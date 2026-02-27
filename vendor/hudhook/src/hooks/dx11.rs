//! Hooks for DirectX 11.

use std::ffi::c_void;
use std::mem;
use std::sync::OnceLock;

use imgui::Context;
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use tracing::{error, trace};
use windows::core::{Error, Interface, Result, HRESULT};
use windows::Win32::Foundation::BOOL;
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_NULL, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDeviceAndSwapChain, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D,
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGISwapChain, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};

use super::DummyHwnd;
use crate::mh::MhHook;
use crate::renderer::{D3D11RenderEngine, Pipeline};
use crate::{util, Hooks, ImguiRenderLoop};

// ─── Inline diagnostic logger (mirrors dx12.rs) ──────────────────────────────
#[link(name = "kernel32")]
extern "system" {
    fn CreateFileA(p: *const u8, access: u32, share: u32, sa: *const c_void, disp: u32, flags: u32, tmpl: *const c_void) -> isize;
    fn SetFilePointer(h: isize, dist: i32, hi: *mut i32, method: u32) -> u32;
    fn WriteFile(h: isize, buf: *const u8, n: u32, written: *mut u32, ov: *const c_void) -> i32;
    fn CloseHandle(h: isize) -> i32;
}

fn hk11_log(msg: &str) {
    static PATH: &[u8]   = b"C:\\Users\\mouss\\Music\\payload_log.txt\0";
    static PREFIX: &[u8] = b"[DX11HOOK] ";
    static NL: &[u8]     = b"\r\n";
    unsafe {
        let h = CreateFileA(PATH.as_ptr(), 0x4000_0000, 3, core::ptr::null(), 4, 0x80, core::ptr::null());
        if h == -1 { return; }
        SetFilePointer(h, 0, core::ptr::null_mut(), 2);
        let mut w = 0u32;
        WriteFile(h, PREFIX.as_ptr(), PREFIX.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, msg.as_ptr(), msg.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, NL.as_ptr(), NL.len() as u32, &mut w, core::ptr::null());
        CloseHandle(h);
    }
}

fn hk11_log_hex(prefix: &str, val: u32) {
    let digits = b"0123456789abcdef";
    let mut buf = [b'0'; 10];
    buf[0] = b'0'; buf[1] = b'x';
    for i in 0u32..8 { buf[(2+i) as usize] = digits[((val >> (28 - i*4)) & 0xf) as usize]; }
    static PATH: &[u8]   = b"C:\\Users\\mouss\\Music\\payload_log.txt\0";
    static PREFIX_B: &[u8] = b"[DX11HOOK] ";
    static NL: &[u8]     = b"\r\n";
    static SP: &[u8]     = b" ";
    unsafe {
        let h = CreateFileA(PATH.as_ptr(), 0x4000_0000, 3, core::ptr::null(), 4, 0x80, core::ptr::null());
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
    unsafe extern "system" fn(This: IDXGISwapChain, SyncInterval: u32, Flags: u32) -> HRESULT;

type DXGISwapChainPresent1Type = unsafe extern "system" fn(
    This: IDXGISwapChain,
    SyncInterval: u32,
    PresentFlags: u32,
    pPresentParameters: *const std::ffi::c_void,
) -> HRESULT;

struct Trampolines {
    dxgi_swap_chain_present: DXGISwapChainPresentType,
    dxgi_swap_chain_present1: DXGISwapChainPresent1Type,
}

static mut TRAMPOLINES: OnceLock<Trampolines> = OnceLock::new();
static mut PIPELINE: OnceCell<Mutex<Pipeline<D3D11RenderEngine>>> = OnceCell::new();
static mut RENDER_LOOP: OnceCell<Box<dyn ImguiRenderLoop + Send + Sync>> = OnceCell::new();

unsafe fn init_pipeline(swap_chain: &IDXGISwapChain) -> Result<Mutex<Pipeline<D3D11RenderEngine>>> {
    hk11_log("init_pipeline: GetDesc...");
    let hwnd = util::try_out_param(|v| swap_chain.GetDesc(v)).map(|desc| desc.OutputWindow)?;

    // ensure_thread_init() has been called on this render thread (via before_render/initialize),
    // and the IAT was fixed in-process before CRT startup, so Context::create() is safe here.
    hk11_log("init_pipeline: calling Context::create()...");
    let gim_before = imgui::sys::igGetCurrentContext() as usize;
    hk11_log_hex("init_pipeline: GImGui_lo", gim_before as u32);
    if gim_before != 0 { imgui::sys::igSetCurrentContext(core::ptr::null_mut()); }
    let mut ctx: Context = Context::create();
    hk11_log("init_pipeline: Context::create OK");
    hk11_log("init_pipeline: GetDevice...");
    let device = swap_chain.GetDevice()?;
    hk11_log("init_pipeline: D3D11RenderEngine::new...");
    let engine = D3D11RenderEngine::new(&device, &mut ctx)?;
    hk11_log("init_pipeline: engine OK");

    let Some(render_loop) = RENDER_LOOP.take() else {
        error!("Render loop not yet initialized");
        return Err(Error::from_hresult(HRESULT(-1)));
    };

    hk11_log("init_pipeline: Pipeline::new...");
    let pipeline = Pipeline::new(hwnd, ctx, engine, render_loop).map_err(|(e, render_loop)| {
        RENDER_LOOP.get_or_init(move || render_loop);
        e
    })?;
    hk11_log("init_pipeline: pipeline OK");

    Ok(Mutex::new(pipeline))
}

fn render(swap_chain: &IDXGISwapChain) -> Result<()> {
    unsafe {
        hk11_log("render: get_or_try_init pipeline...");
        let pipeline = PIPELINE.get_or_try_init(|| init_pipeline(swap_chain))?;
        hk11_log("render: try_lock...");

        let Some(mut pipeline) = pipeline.try_lock() else {
            error!("Could not lock pipeline");
            return Err(Error::from_hresult(HRESULT(-1)));
        };

        hk11_log("render: prepare_render...");
        pipeline.prepare_render()?;
        hk11_log("render: GetBuffer(0)...");
        let target: ID3D11Texture2D = swap_chain.GetBuffer(0)?;
        hk11_log("render: pipeline.render...");
        pipeline.render(target)?;
        hk11_log("render: OK");
    }
    Ok(())
}

unsafe extern "system" fn dxgi_swap_chain_present_impl(
    swap_chain: IDXGISwapChain,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    static CNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = CNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n < 3 { hk11_log("Present hook reached"); }

    let Trampolines { dxgi_swap_chain_present, .. } =
        TRAMPOLINES.get().expect("DirectX 11 trampolines uninitialized");

    if let Err(e) = render(&swap_chain) {
        static RLOG: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !RLOG.swap(true, std::sync::atomic::Ordering::Relaxed) {
            hk11_log_hex("Present: render ERR HRESULT", e.code().0 as u32);
        }
        error!("Render error: {e:?}");
    }

    trace!("Call IDXGISwapChain::Present trampoline");
    dxgi_swap_chain_present(swap_chain, sync_interval, flags)
}

unsafe extern "system" fn dxgi_swap_chain_present1_impl(
    swap_chain: IDXGISwapChain,
    sync_interval: u32,
    present_flags: u32,
    p_present_parameters: *const std::ffi::c_void,
) -> HRESULT {
    static CNT1: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n1 = CNT1.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    if n1 < 3 { hk11_log("Present1 hook reached"); }

    let Trampolines { dxgi_swap_chain_present1, .. } =
        TRAMPOLINES.get().expect("DirectX 11 trampolines uninitialized");

    if let Err(e) = render(&swap_chain) {
        static RLOG1: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !RLOG1.swap(true, std::sync::atomic::Ordering::Relaxed) {
            hk11_log_hex("Present1: render ERR HRESULT", e.code().0 as u32);
        }
        error!("Render error (Present1): {e:?}");
    }

    dxgi_swap_chain_present1(swap_chain, sync_interval, present_flags, p_present_parameters)
}

fn get_target_addrs() -> (DXGISwapChainPresentType, Option<DXGISwapChainPresent1Type>) {
    let mut p_device: Option<ID3D11Device> = None;
    let mut p_context: Option<ID3D11DeviceContext> = None;
    let mut p_swap_chain: Option<IDXGISwapChain> = None;

    let dummy_hwnd = DummyHwnd::new();
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_NULL,
            None,
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC {
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
                    Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
                    ..Default::default()
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 1,
                OutputWindow: dummy_hwnd.hwnd(),
                Windowed: BOOL(1),
                SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, ..Default::default() },
                ..Default::default()
            }),
            Some(&mut p_swap_chain),
            Some(&mut p_device),
            None,
            Some(&mut p_context),
        )
        .expect("D3D11CreateDeviceAndSwapChain failed");
    }

    let swap_chain = p_swap_chain.unwrap();

    let present_ptr: DXGISwapChainPresentType = unsafe {
        mem::transmute::<
            unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT,
            DXGISwapChainPresentType,
        >(swap_chain.vtable().Present)
    };

    hk11_log_hex("get_target_addrs: Present  addr lo", (present_ptr as usize) as u32);
    hk11_log_hex("get_target_addrs: Present  addr hi", ((present_ptr as usize) >> 32) as u32);

    // Present1 is at vtable slot 22 of IDXGISwapChain1.
    // The IDXGISwapChain returned by D3D11CreateDeviceAndSwapChain implements
    // IDXGISwapChain1 when running on DXGI 1.1+, so slot 22 is valid.
    let present1_ptr: Option<DXGISwapChainPresent1Type> = unsafe {
        let vtbl = *(swap_chain.as_raw() as *const *const usize);
        let fn_ptr = *vtbl.add(22);
        if fn_ptr > 0x1000 {
            hk11_log_hex("get_target_addrs: Present1 addr lo", fn_ptr as u32);
            hk11_log_hex("get_target_addrs: Present1 addr hi", (fn_ptr >> 32) as u32);
            Some(mem::transmute(fn_ptr))
        } else {
            hk11_log("get_target_addrs: Present1 slot invalid");
            None
        }
    };

    (present_ptr, present1_ptr)
}

unsafe extern "system" fn dxgi_swap_chain_present1_passthrough(
    _: IDXGISwapChain, _: u32, _: u32, _: *const std::ffi::c_void,
) -> HRESULT { HRESULT(0) }

/// Hooks for DirectX 11.
pub struct ImguiDx11Hooks(Vec<MhHook>);

impl ImguiDx11Hooks {
    /// Construct a set of [`MhHook`]s that will render UI via the
    /// provided [`ImguiRenderLoop`].
    ///
    /// The following functions are hooked:
    /// - `IDXGISwapChain::Present`
    /// - `IDXGISwapChain1::Present1` (if available)
    ///
    /// # Safety
    ///
    /// yolo
    pub unsafe fn new<T>(t: T) -> Self
    where
        T: ImguiRenderLoop + Send + Sync + 'static,
    {
        let (dxgi_swap_chain_present_addr, dxgi_swap_chain_present1_addr) = get_target_addrs();

        let hook_present = MhHook::new(
            dxgi_swap_chain_present_addr as *mut _,
            dxgi_swap_chain_present_impl as *mut _,
        )
        .expect("couldn't create IDXGISwapChain::Present hook");

        let hook_present1: Option<MhHook> = if let Some(p1) = dxgi_swap_chain_present1_addr {
            match MhHook::new(p1 as *mut _, dxgi_swap_chain_present1_impl as *mut _) {
                Ok(h) => { hk11_log("Present1 hook installed OK"); Some(h) }
                Err(_) => { hk11_log("Present1 hook FAILED"); None }
            }
        } else { None };

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
        });

        // Pre-create the imgui Context here on the hook thread.
        // ── Win32-heap allocators for imgui ───────────────────────────────
        // With the IAT fixed in-process (by fix_iat() in imruski_init before CRT startup),
        // VCRUNTIME140/d3d11/etc. are correctly resolved in the game's address space.
        // Context::create() (which calls igCreateContext via the CRT allocators) will
        // therefore work on the render thread inside init_pipeline() once
        // ensure_thread_init() has been called there.
        // We leave PREBUILT_CTX empty; init_pipeline falls through to Context::create().
        hk11_log("Hooks::new: hooks installed, context will be created on render thread");

        let mut hooks = vec![hook_present];
        if let Some(h1) = hook_present1 { hooks.push(h1); }
        Self(hooks)
    }
}

impl Hooks for ImguiDx11Hooks {
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
        RENDER_LOOP.take(); // should already be null
    }
}
