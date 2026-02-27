//! Hooks for DirectX 12.

use std::ffi::c_void;
use std::mem;
use std::sync::OnceLock;

use imgui::Context;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use tracing::{error, trace};
use windows::core::{Error, Interface, Result, HRESULT};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0;
use windows::Win32::Graphics::Direct3D12::{
    D3D12CreateDevice, ID3D12CommandList, ID3D12CommandQueue, ID3D12Device, ID3D12Resource,
    D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_QUEUE_DESC, D3D12_COMMAND_QUEUE_FLAG_NONE,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_RATIONAL, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIFactory2, IDXGISwapChain, IDXGISwapChain1, IDXGISwapChain3,
    DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC,
    DXGI_SWAP_CHAIN_DESC1, DXGI_SWAP_CHAIN_FLAG_ALLOW_MODE_SWITCH,
    DXGI_SWAP_EFFECT_FLIP_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_ALPHA_MODE_IGNORE;

use super::DummyHwnd;
use crate::mh::MhHook;
use crate::renderer::{D3D12RenderEngine, Pipeline};
use crate::{util, Hooks, ImguiRenderLoop};

// ─── Inline diagnostic logger ────────────────────────────────────────────────
// Writes directly to the payload log file using raw Win32 so it works from
// any thread at any point in execution.
#[link(name = "kernel32")]
extern "system" {
    fn CreateFileA(
        p: *const u8, access: u32, share: u32,
        sa: *const c_void, disp: u32, flags: u32,
        tmpl: *const c_void,
    ) -> isize;
    fn SetFilePointer(h: isize, dist: i32, hi: *mut i32, method: u32) -> u32;
    fn WriteFile(h: isize, buf: *const u8, n: u32, written: *mut u32,
                 ov: *const c_void) -> i32;
    fn CloseHandle(h: isize) -> i32;
}

fn hk_log(msg: &str) {
    static PATH: &[u8]   = b"C:\\Users\\mouss\\Music\\payload_log.txt\0";
    static PREFIX: &[u8] = b"[HUDHOOK] ";
    static NL: &[u8]     = b"\r\n";
    unsafe {
        let h = CreateFileA(PATH.as_ptr(), 0x4000_0000, 3,
                            core::ptr::null(), 4, 0x80, core::ptr::null());
        if h == -1 { return; }
        SetFilePointer(h, 0, core::ptr::null_mut(), 2);
        let mut w = 0u32;
        WriteFile(h, PREFIX.as_ptr(), PREFIX.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, msg.as_ptr(), msg.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, NL.as_ptr(), NL.len() as u32, &mut w, core::ptr::null());
        CloseHandle(h);
    }
}

// Log prefix + " 0x" + 8-digit hex of val. No heap.
fn hk_log_hex(prefix: &str, val: u32) {
    let digits = b"0123456789abcdef";
    let mut buf = [b'0'; 10]; // "0x" + 8 hex digits
    buf[0] = b'0'; buf[1] = b'x';
    for i in 0u32..8 {
        buf[(2 + i) as usize] = digits[((val >> (28 - i * 4)) & 0xf) as usize];
    }
    static PATH: &[u8]   = b"C:\\Users\\mouss\\Music\\payload_log.txt\0";
    static PREFIX_B: &[u8] = b"[HUDHOOK] ";
    static NL: &[u8]     = b"\r\n";
    static SP: &[u8]     = b" ";
    unsafe {
        let h = CreateFileA(PATH.as_ptr(), 0x4000_0000, 3,
                            core::ptr::null(), 4, 0x80, core::ptr::null());
        if h == -1 { return; }
        SetFilePointer(h, 0, core::ptr::null_mut(), 2);
        let mut w = 0u32;
        WriteFile(h, PREFIX_B.as_ptr(), PREFIX_B.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, prefix.as_ptr(), prefix.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, SP.as_ptr(), 1, &mut w, core::ptr::null());
        WriteFile(h, buf.as_ptr(), 10, &mut w, core::ptr::null());
        WriteFile(h, NL.as_ptr(), NL.len() as u32, &mut w, core::ptr::null());
        CloseHandle(h);
    }
}

type DXGISwapChainPresentType =
    unsafe extern "system" fn(This: IDXGISwapChain3, SyncInterval: u32, Flags: u32) -> HRESULT;

// IDXGISwapChain1::Present1 — vtable slot 22.
// Unity DX12 (and other modern apps) call Present1 instead of Present.
type DXGISwapChainPresent1Type = unsafe extern "system" fn(
    This: IDXGISwapChain3,
    SyncInterval: u32,
    PresentFlags: u32,
    pPresentParameters: *const c_void,
) -> HRESULT;

type DXGISwapChainResizeBuffersType = unsafe extern "system" fn(
    This: IDXGISwapChain3,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT;

type D3D12CommandQueueExecuteCommandListsType = unsafe extern "system" fn(
    This: ID3D12CommandQueue,
    num_command_lists: u32,
    command_lists: *mut ID3D12CommandList,
);

struct Trampolines {
    dxgi_swap_chain_present: DXGISwapChainPresentType,
    dxgi_swap_chain_present1: DXGISwapChainPresent1Type,
    dxgi_swap_chain_resize_buffers: DXGISwapChainResizeBuffersType,
    d3d12_command_queue_execute_command_lists: D3D12CommandQueueExecuteCommandListsType,
}

static mut TRAMPOLINES: OnceLock<Trampolines> = OnceLock::new();

static mut PIPELINE: OnceCell<Mutex<Pipeline<D3D12RenderEngine>>> = OnceCell::new();
static mut COMMAND_QUEUE: OnceCell<ID3D12CommandQueue> = OnceCell::new();
static mut RENDER_LOOP: OnceCell<Box<dyn ImguiRenderLoop + Send + Sync>> = OnceCell::new();

unsafe fn init_pipeline(
    swap_chain: &IDXGISwapChain3,
) -> Result<Mutex<Pipeline<D3D12RenderEngine>>> {
    let Some(command_queue) = COMMAND_QUEUE.get() else {
        error!("Command queue not yet initialized");
        return Err(Error::from_hresult(HRESULT(-1)));
    };

    let hwnd = util::try_out_param(|v| swap_chain.GetDesc(v)).map(|desc| desc.OutputWindow)?;

    let mut ctx = Context::create();
    let engine = D3D12RenderEngine::new(command_queue, &mut ctx)?;

    let Some(render_loop) = RENDER_LOOP.take() else {
        error!("Render loop not yet initialized");
        return Err(Error::from_hresult(HRESULT(-1)));
    };

    let pipeline = Pipeline::new(hwnd, ctx, engine, render_loop).map_err(|(e, render_loop)| {
        RENDER_LOOP.get_or_init(move || render_loop);
        e
    })?;

    Ok(Mutex::new(pipeline))
}

fn render(swap_chain: &IDXGISwapChain3) -> Result<()> {
    unsafe {
        let pipeline = PIPELINE.get_or_try_init(|| init_pipeline(swap_chain))?;

        let Some(mut pipeline) = pipeline.try_lock() else {
            error!("Could not lock pipeline");
            return Err(Error::from_hresult(HRESULT(-1)));
        };

        pipeline.prepare_render()?;

        let target: ID3D12Resource =
            swap_chain.GetBuffer(swap_chain.GetCurrentBackBufferIndex())?;

        pipeline.render(target)?;
    }

    Ok(())
}

unsafe extern "system" fn bootstrap_command_queue(swap_chain: &IDXGISwapChain3) {
    if COMMAND_QUEUE.get().is_some() { return; }
    // Run the full diagnostic exactly once, then decide whether to keep trying.
    static DIAG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if DIAG.swap(true, std::sync::atomic::Ordering::Relaxed) {
        // After the first diagnostic, keep retrying GetBuffer silently in case
        // the first call was a DX11 swap chain and a DX12 one appears later.
        if let Ok(res) = swap_chain.GetBuffer::<ID3D12Resource>(0) {
            static LOGGED_CQ: std::sync::atomic::AtomicBool =
                std::sync::atomic::AtomicBool::new(false);
            if let Ok(d) = util::try_out_ptr(|v| res.GetDevice(v)) {
                let d: ID3D12Device = d;
                if let Ok(cq) = d.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    Priority: 0,
                    Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                    NodeMask: 0,
                }) {
                    hk_log("bootstrap_cq: COMMAND_QUEUE ready");
                    let _ = COMMAND_QUEUE.get_or_init(|| cq);
                } else if !LOGGED_CQ.swap(true, std::sync::atomic::Ordering::Relaxed) {
                    hk_log("bootstrap_cq: CreateCommandQueue failed");
                }
            }
        }
        return;
    }

    // ── One-shot diagnostic ──────────────────────────────────────────────────
    // Log the vtable pointer of the swap chain so we can identify the module.
    let vtbl: *const usize = *(swap_chain as *const IDXGISwapChain3 as *const *const usize);
    hk_log_hex("diag: swapchain vtbl lo", vtbl as u32);
    hk_log_hex("diag: swapchain vtbl hi", (vtbl as usize >> 32) as u32);

    // Try GetBuffer<ID3D12Resource> — works if this is a DX12 swap chain.
    match swap_chain.GetBuffer::<ID3D12Resource>(0) {
        Ok(res) => {
            hk_log("diag: GetBuffer<ID3D12Resource> OK  -> DX12 swap chain");
            // Got the DX12 resource → proceed to get device & create queue.
            match util::try_out_ptr(|v| res.GetDevice(v)) {
                Ok(device) => {
                    let device: ID3D12Device = device;
                    hk_log("diag: resource.GetDevice OK");
                    match device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                        Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                        Priority: 0,
                        Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
                        NodeMask: 0,
                    }) {
                        Ok(cq) => {
                            hk_log("bootstrap_cq: COMMAND_QUEUE ready");
                            let _ = COMMAND_QUEUE.get_or_init(|| cq);
                        }
                        Err(e) => hk_log_hex("diag: CreateCommandQueue FAIL", e.code().0 as u32),
                    }
                }
                Err(e) => hk_log_hex("diag: resource.GetDevice FAIL", e.code().0 as u32),
            }
        }
        Err(e) => {
            hk_log_hex("diag: GetBuffer<ID3D12Resource> FAIL (E_NOINTERFACE=80004002)", e.code().0 as u32);

            // Try IDXGISurface — works for DX9/DX11/DX12 swap chains.
            use windows::Win32::Graphics::Dxgi::IDXGISurface;
            match swap_chain.GetBuffer::<IDXGISurface>(0) {
                Ok(_) => hk_log("diag: GetBuffer<IDXGISurface>  OK  -> DX11 (or DX9) swap chain"),
                Err(e2) => hk_log_hex("diag: GetBuffer<IDXGISurface>  FAIL", e2.code().0 as u32),
            }

            // Try DXGI sub-object GetDevice<ID3D12Device>.
            use windows::Win32::Graphics::Dxgi::IDXGIDeviceSubObject;
            match swap_chain.cast::<IDXGIDeviceSubObject>() {
                Ok(sub) => match sub.GetDevice::<ID3D12Device>() {
                    Ok(_) => hk_log("diag: DXGI GetDevice<ID3D12Device> OK"),
                    Err(e3) => hk_log_hex("diag: DXGI GetDevice<ID3D12Device> FAIL", e3.code().0 as u32),
                },
                Err(e3) => hk_log_hex("diag: cast IDXGIDeviceSubObject FAIL", e3.code().0 as u32),
            }

            // Try DXGI sub-object GetDevice<ID3D11Device> (confirms DX11 swap chain).
            use windows::Win32::Graphics::Direct3D11::ID3D11Device;
            match swap_chain.cast::<IDXGIDeviceSubObject>() {
                Ok(sub) => match sub.GetDevice::<ID3D11Device>() {
                    Ok(_) => hk_log("diag: DXGI GetDevice<ID3D11Device> OK  -> DX11 device"),
                    Err(e3) => hk_log_hex("diag: DXGI GetDevice<ID3D11Device> FAIL", e3.code().0 as u32),
                },
                _ => {}
            }
        }
    }
}

unsafe extern "system" fn dxgi_swap_chain_present_impl(
    swap_chain: IDXGISwapChain3,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    static CNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n < 3 { hk_log("Present hook reached"); }
    static FIRED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !FIRED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        hk_log("Present FIRED (first call)");
    }
    bootstrap_command_queue(&swap_chain);
    let Trampolines { dxgi_swap_chain_present, .. } =
        TRAMPOLINES.get().expect("DirectX 12 trampolines uninitialized");

    if let Err(e) = render(&swap_chain) {
        util::print_dxgi_debug_messages();
        error!("Render error: {e:?}");
    }

    trace!("Call IDXGISwapChain::Present trampoline");
    dxgi_swap_chain_present(swap_chain, sync_interval, flags)
}

unsafe extern "system" fn dxgi_swap_chain_present1_impl(
    swap_chain: IDXGISwapChain3,
    sync_interval: u32,
    present_flags: u32,
    p_present_parameters: *const c_void,
) -> HRESULT {
    static CNT1: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n1 = CNT1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n1 < 3 { hk_log("Present1 hook reached"); }
    static FIRED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !FIRED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        hk_log("Present1 FIRED (first call)");
    }
    bootstrap_command_queue(&swap_chain);
    let Trampolines { dxgi_swap_chain_present1, .. } =
        TRAMPOLINES.get().expect("DirectX 12 trampolines uninitialized");

    if let Err(e) = render(&swap_chain) {
        util::print_dxgi_debug_messages();
        error!("Render error (Present1): {e:?}");
    }

    trace!("Call IDXGISwapChain1::Present1 trampoline");
    dxgi_swap_chain_present1(swap_chain, sync_interval, present_flags, p_present_parameters)
}

unsafe extern "system" fn dxgi_swap_chain_resize_buffers_impl(
    p_this: IDXGISwapChain3,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT {
    let Trampolines { dxgi_swap_chain_resize_buffers, .. } =
        TRAMPOLINES.get().expect("DirectX 12 trampolines uninitialized");

    trace!("Call IDXGISwapChain::ResizeBuffers trampoline");
    dxgi_swap_chain_resize_buffers(p_this, buffer_count, width, height, new_format, flags)
}

unsafe extern "system" fn d3d12_command_queue_execute_command_lists_impl(
    command_queue: ID3D12CommandQueue,
    num_command_lists: u32,
    command_lists: *mut ID3D12CommandList,
) {
    static FIRED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !FIRED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        hk_log("ECL FIRED (first call)");
    }
    trace!(
        "ID3D12CommandQueue::ExecuteCommandLists({command_queue:?}, {num_command_lists}, \
         {command_lists:p}) invoked",
    );

    let Trampolines { d3d12_command_queue_execute_command_lists, .. } =
        TRAMPOLINES.get().expect("DirectX 12 trampolines uninitialized");

    COMMAND_QUEUE
        .get_or_try_init(|| unsafe {
            let desc = command_queue.GetDesc();
            if desc.Type == D3D12_COMMAND_LIST_TYPE_DIRECT {
                Ok(command_queue.clone())
            } else {
                Err(())
            }
        })
        .ok();

    d3d12_command_queue_execute_command_lists(command_queue, num_command_lists, command_lists);
}

fn get_target_addrs() -> (
    DXGISwapChainPresentType,
    Option<DXGISwapChainPresent1Type>,
    DXGISwapChainResizeBuffersType,
    D3D12CommandQueueExecuteCommandListsType,
) {
    hk_log("get_target_addrs: start");
    let dummy_hwnd = DummyHwnd::new();
    hk_log("get_target_addrs: DummyHwnd OK");

    let factory: IDXGIFactory2 =
        unsafe { CreateDXGIFactory2(0) }.unwrap();
    hk_log("get_target_addrs: factory OK");
    let adapter = unsafe { factory.EnumAdapters(0) }.unwrap();
    hk_log("get_target_addrs: adapter OK");

    let device: ID3D12Device =
        util::try_out_ptr(|v| unsafe { D3D12CreateDevice(&adapter, D3D_FEATURE_LEVEL_11_0, v) })
            .expect("D3D12CreateDevice failed");
    hk_log("get_target_addrs: device OK");

    let command_queue: ID3D12CommandQueue = unsafe {
        device.CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
            Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
            Priority: 0,
            Flags: D3D12_COMMAND_QUEUE_FLAG_NONE,
            NodeMask: 0,
        })
    }
    .unwrap();
    hk_log("get_target_addrs: command_queue OK");

    // Always use the modern CreateSwapChainForHwnd path.
    // Use the DUMMY HWND — the game's HWND already has Unity's swapchain
    // attached, so creating a second one on it fails with INVALID_CALL.
    let hwnd = dummy_hwnd.hwnd();
    hk_log("get_target_addrs: hwnd fetched");

    let sc_desc1 = DXGI_SWAP_CHAIN_DESC1 {
        Width: 64,
        Height: 64,
        Format: DXGI_FORMAT_R8G8B8A8_UNORM,
        Stereo: BOOL(0),
        SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        Scaling: DXGI_SCALING_STRETCH,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        AlphaMode: DXGI_ALPHA_MODE_IGNORE,
        Flags: 0,
    };

    hk_log("get_target_addrs: calling CreateSwapChainForHwnd...");
    let sc1: IDXGISwapChain1 = match unsafe {
        factory.CreateSwapChainForHwnd(&command_queue, hwnd, &sc_desc1, None, None)
    } {
        Ok(sc) => {
            hk_log("get_target_addrs: CreateSwapChainForHwnd OK");
            sc
        }
        Err(e) => {
            hk_log(&format!("get_target_addrs: CreateSwapChainForHwnd FAILED: {e:?}"));
            util::print_dxgi_debug_messages();
            // Without a swapchain we can't get vtable addresses — bail out with
            // null-equivalent trampolines so the game doesn't crash.
            unsafe extern "system" fn null_present(
                _: IDXGISwapChain, _: u32, _: u32,
            ) -> HRESULT { HRESULT(0) }
            unsafe extern "system" fn null_resize(
                _: IDXGISwapChain, _: u32, _: u32, _: u32, _: DXGI_FORMAT, _: u32,
            ) -> HRESULT { HRESULT(0) }
            unsafe extern "system" fn null_ecl(
                _: ID3D12CommandQueue, _: u32, _: *mut Option<ID3D12CommandList>,
            ) { }
            return (
                unsafe { mem::transmute::<_, DXGISwapChainPresentType>(null_present as usize) },
                None,
                unsafe { mem::transmute::<_, DXGISwapChainResizeBuffersType>(null_resize as usize) },
                unsafe { mem::transmute::<_, D3D12CommandQueueExecuteCommandListsType>(null_ecl as usize) },
            );
        }
    };

    // Cast to IDXGISwapChain to read Present/ResizeBuffers vtable fields
    // (they're defined on IDXGISwapChain, not IDXGISwapChain1).
    let sc: IDXGISwapChain = sc1.cast().expect("IDXGISwapChain1 → IDXGISwapChain cast failed");

    let present_ptr: DXGISwapChainPresentType =
        unsafe { mem::transmute(sc.vtable().Present) };
    let resize_buffers_ptr: DXGISwapChainResizeBuffersType =
        unsafe { mem::transmute(sc.vtable().ResizeBuffers) };

    // Present1 is at slot 22 of IDXGISwapChain1's vtable.
    // Read it directly from the IDXGISwapChain1 vtable (not IDXGISwapChain).
    let present1_ptr: Option<DXGISwapChainPresent1Type> = unsafe {
        let vtbl = *(sc1.as_raw() as *const *const usize);
        let fn_ptr = *vtbl.add(22);
        // Slot 22 is valid only if the object implements IDXGISwapChain1+.
        // A fn pointer of 0 or in unmapped memory would be wrong; accept anything
        // that looks like a plausible code address (non-zero, high canonical form).
        if fn_ptr > 0x1000 {
            Some(mem::transmute(fn_ptr))
        } else {
            hk_log("get_target_addrs: Present1 slot invalid, skipping");
            None
        }
    };

    let cqecl_ptr: D3D12CommandQueueExecuteCommandListsType =
        unsafe { mem::transmute(command_queue.vtable().ExecuteCommandLists) };

    hk_log_hex("get_target_addrs: Present addr lo", (present_ptr as usize) as u32);
    hk_log_hex("get_target_addrs: Present addr hi", ((present_ptr as usize) >> 32) as u32);
    hk_log_hex("get_target_addrs: CQECL  addr lo", (cqecl_ptr as usize) as u32);
    hk_log_hex("get_target_addrs: CQECL  addr hi", ((cqecl_ptr as usize) >> 32) as u32);
    hk_log("get_target_addrs: got vtable addrs OK");
    if let Some(p1) = present1_ptr {
        hk_log_hex("get_target_addrs: Present1 addr lo", (p1 as usize) as u32);
        hk_log_hex("get_target_addrs: Present1 addr hi", ((p1 as usize) >> 32) as u32);
        hk_log("get_target_addrs: Present1 slot valid");
    }

    (present_ptr, present1_ptr, resize_buffers_ptr, cqecl_ptr)
}

// Passthrough used when Present1 hook couldn't be installed.
// The trampoline field still needs a valid function pointer.
unsafe extern "system" fn dxgi_swap_chain_present1_passthrough(
    _swap_chain: IDXGISwapChain3,
    _sync_interval: u32,
    _present_flags: u32,
    _p_present_parameters: *const c_void,
) -> HRESULT {
    HRESULT(0)
}

/// Hooks for DirectX 12.
pub struct ImguiDx12Hooks(Vec<MhHook>);

impl ImguiDx12Hooks {
    /// Construct a set of [`MhHook`]s that will render UI via the
    /// provided [`ImguiRenderLoop`].
    ///
    /// The following functions are hooked:
    /// - `IDXGISwapChain3::Present`
    /// - `IDXGISwapChain3::ResizeBuffers`
    /// - `ID3D12CommandQueue::ExecuteCommandLists`
    ///
    /// # Safety
    ///
    /// yolo
    pub unsafe fn new<T>(t: T) -> Self
    where
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        let (
            dxgi_swap_chain_present_addr,
            dxgi_swap_chain_present1_addr,
            dxgi_swap_chain_resize_buffers_addr,
            d3d12_command_queue_execute_command_lists_addr,
        ) = get_target_addrs();

        trace!("IDXGISwapChain::Present  = {:p}", dxgi_swap_chain_present_addr as *const c_void);
        let hook_present = MhHook::new(
            dxgi_swap_chain_present_addr as *mut _,
            dxgi_swap_chain_present_impl as *mut _,
        )
        .expect("couldn't create IDXGISwapChain::Present hook");

        // Present1 hook is optional — only install if we got a valid address
        // and MinHook can patch it (non-panicking).
        let hook_present1: Option<MhHook> =
            if let Some(p1_addr) = dxgi_swap_chain_present1_addr {
                trace!("IDXGISwapChain1::Present1 = {:p}", p1_addr as *const c_void);
                match MhHook::new(
                    p1_addr as *mut _,
                    dxgi_swap_chain_present1_impl as *mut _,
                ) {
                    Ok(h) => {
                        hk_log("Present1 hook installed OK");
                        Some(h)
                    }
                    Err(e) => {
                        hk_log(&format!("Present1 hook FAILED (skipping): {e:?}"));
                        None
                    }
                }
            } else {
                None
            };

        let hook_resize_buffers = MhHook::new(
            dxgi_swap_chain_resize_buffers_addr as *mut _,
            dxgi_swap_chain_resize_buffers_impl as *mut _,
        )
        .expect("couldn't create IDXGISwapChain::ResizeBuffers hook");
        let hook_cqecl = MhHook::new(
            d3d12_command_queue_execute_command_lists_addr as *mut _,
            d3d12_command_queue_execute_command_lists_impl as *mut _,
        )
        .expect("couldn't create ID3D12CommandQueue::ExecuteCommandLists hook");

        RENDER_LOOP.get_or_init(|| Box::new(t));

        TRAMPOLINES.get_or_init(|| Trampolines {
            dxgi_swap_chain_present: mem::transmute::<*mut c_void, DXGISwapChainPresentType>(
                hook_present.trampoline(),
            ),
            dxgi_swap_chain_present1: mem::transmute::<*mut c_void, DXGISwapChainPresent1Type>(
                hook_present1.as_ref()
                    .map(|h| h.trampoline())
                    .unwrap_or(dxgi_swap_chain_present1_passthrough as *mut _),
            ),
            dxgi_swap_chain_resize_buffers: mem::transmute::<
                *mut c_void,
                DXGISwapChainResizeBuffersType,
            >(hook_resize_buffers.trampoline()),
            d3d12_command_queue_execute_command_lists: mem::transmute::<
                *mut c_void,
                D3D12CommandQueueExecuteCommandListsType,
            >(hook_cqecl.trampoline()),
        });

        let mut hooks = vec![hook_present, hook_resize_buffers, hook_cqecl];
        if let Some(h1) = hook_present1 { hooks.push(h1); }
        Self(hooks)
    }
}

impl Hooks for ImguiDx12Hooks {
    fn from_render_loop<T>(t: T) -> Box<Self>
    where
        Self: Sized,
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        Box::new(unsafe { Self::new(t) })
    }

    fn hooks(&self) -> &[MhHook] {
        &self.0
    }

    unsafe fn unhook(&mut self) {
        TRAMPOLINES.take();
        PIPELINE.take().map(|p| p.into_inner().take());
        COMMAND_QUEUE.take();
        RENDER_LOOP.take(); // should already be null
    }
}
