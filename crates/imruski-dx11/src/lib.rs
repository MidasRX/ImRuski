//! # imruski-dx11
//!
//! DirectX 11 game-overlay backend for **ImRuski**.
//! Hooks `IDXGISwapChain::Present` via hudhook, converts the imruski
//! `DrawList` to imgui background-draw-list calls each frame.

#![cfg(windows)]

/// Re-export the imgui crate so downstream crates don't need a direct
/// dependency on hudhook just to access `imgui::Context` / `StyleColor` etc.
pub use hudhook::imgui;

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::mem;
use imruski_core::{
    draw_list::TextureId,
    renderer::{FontAtlas, GlyphInfo, RenderFrame},
    Context, Vec2,
};
use std::sync::{Arc, Mutex};

// ─── Raw Win32 logging (works before std/CRT is fully initialised) ────────────
#[allow(non_snake_case)]
extern "system" {
    fn CreateFileA(
        p: *const u8, access: u32, share: u32,
        sa: *const std::ffi::c_void, disp: u32, flags: u32,
        tmpl: *const std::ffi::c_void,
    ) -> isize;
    fn SetFilePointer(h: isize, dist: i32, hi: *mut i32, method: u32) -> u32;
    fn WriteFile(h: isize, buf: *const u8, n: u32, written: *mut u32,
                 ov: *const std::ffi::c_void) -> i32;
    #[link_name = "CloseHandle"]
    fn CloseFileHandle(h: isize) -> i32;
}

fn dx11_log(msg: &str) {
    static PATH: &[u8]   = b"C:\\Users\\mouss\\Music\\payload_log.txt\0";
    static PREFIX: &[u8] = b"[DX11] ";
    static NL: &[u8]     = b"\r\n";
    unsafe {
        let h = CreateFileA(PATH.as_ptr(), 0x4000_0000, 3, std::ptr::null(), 4, 0x80, std::ptr::null());
        if h == -1 { return; }
        SetFilePointer(h, 0, std::ptr::null_mut(), 2);
        let mut w = 0u32;
        WriteFile(h, PREFIX.as_ptr(), PREFIX.len() as u32, &mut w, std::ptr::null());
        WriteFile(h, msg.as_ptr(), msg.len() as u32, &mut w, std::ptr::null());
        WriteFile(h, NL.as_ptr(), NL.len() as u32, &mut w, std::ptr::null());
        CloseFileHandle(h);
    }
}

// ─── Per-thread DLL init (manually-mapped TLS fix) ───────────────────────────
//
// Game render/WndProc threads never receive DLL_THREAD_ATTACH for our
// manually-mapped DLL.  Calling _DllMainCRTStartup(base, 2, 0) from
// the very first Present/initialize callback initialises the CRT
// per-thread data (ptiddata) so MSVC CRT functions work on those threads.
//
// NOTE: Rust 1.76+ on x86_64-pc-windows-msvc uses *dynamic* TLS (TlsAlloc)
// for thread_local!, so this is only needed for MSVC CRT per-thread state
// (errno, strtok buffers, etc.) – but calling it harmlessly when not needed
// causes no side-effects.

/// DLL base address – set once by [`Dx11Hook::new`].
static DLL_BASE: AtomicUsize = AtomicUsize::new(0);
/// DLL entry-point VA  – set once by [`Dx11Hook::new`].
static DLL_EP:   AtomicUsize = AtomicUsize::new(0);
/// Win32 TLS slot used as a per-thread "already initialised" flag.
static INIT_SLOT: AtomicU32 = AtomicU32::new(u32::MAX);

#[link(name = "kernel32")]
extern "system" {
    fn TlsAlloc() -> u32;
    fn TlsFree(slot: u32) -> i32;
    fn TlsGetValue(slot: u32) -> *mut std::ffi::c_void;
    fn TlsSetValue(slot: u32, value: *mut std::ffi::c_void) -> i32;
}

/// Must be called at the top of every callback that runs on a foreign thread.
#[inline]
unsafe fn ensure_thread_init() {
    // ── get or lazily allocate the guard slot ──────────────────────────────
    let slot = {
        let s = INIT_SLOT.load(Ordering::Acquire);
        if s == u32::MAX {
            let new_s = TlsAlloc();
            match INIT_SLOT.compare_exchange(u32::MAX, new_s, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_)          => new_s,
                Err(existing)  => { let _ = TlsFree(new_s); existing }
            }
        } else {
            s
        }
    };
    // ── per-thread check ───────────────────────────────────────────────────
    if TlsGetValue(slot).is_null() {
        let base = DLL_BASE.load(Ordering::Relaxed);
        let ep   = DLL_EP.load(Ordering::Relaxed);
        if base != 0 && ep != 0 {
            let attach: unsafe extern "system" fn(usize, u32, usize) -> i32 =
                mem::transmute(ep);
            attach(base, 2, 0);
        }
        TlsSetValue(slot, 1usize as *mut std::ffi::c_void);
    }
}

//  Error 

#[derive(Debug, thiserror::Error)]
pub enum Dx11Error {
    #[error("Hook installation failed: {0}")]
    HookInstall(String),
}

//  Font atlas 

/// Minimal bitmap-metric font atlas for the DX11 backend.
pub struct Dx11FontAtlas;

impl FontAtlas for Dx11FontAtlas {
    fn glyph(&self, _ch: char, size_px: f32) -> Option<GlyphInfo> {
        let w = size_px * 0.55;
        Some(GlyphInfo {
            uv_min:    Vec2::ZERO,
            uv_max:    Vec2::ZERO,
            size:      Vec2::new(w, size_px),
            advance_x: w,
            offset_y:  0.0,
        })
    }
    fn texture(&self) -> TextureId { TextureId::FONT }
    fn measure(&self, text: &str, size_px: f32) -> f32 {
        text.chars().count() as f32 * size_px * 0.55
    }
}

//  Dx11Frame 

/// Per-frame context passed to [`ImRuskiRenderLoop::render`].
///
/// Call [`frame.ui(ctx)`](Dx11Frame::ui) to obtain an `imruski_core::Ui`,
/// then call [`frame.submit`](Dx11Frame::submit) with the finished frame.
pub struct Dx11Frame<'a> {
    pub(crate) imgui_ui:     &'a mut hudhook::imgui::Ui,
    pub(crate) font:         Dx11FontAtlas,
    pub(crate) display_size: Vec2,
}

impl<'a> Dx11Frame<'a> {
    /// Borrow an imruski `Ui` for this frame.
    pub fn ui<'ctx>(&'ctx self, ctx: &'ctx mut Context) -> imruski_core::ui::Ui<'ctx> {
        imruski_core::ui::Ui::new(ctx, &self.font, 1.0)
    }

    /// Submit the finished [`RenderFrame`] to the screen.
    pub fn submit(&mut self, frame: RenderFrame<'_>) {
        let dl = frame.draw_list;
        if dl.cmd_buf.is_empty() { return; }
        let bg    = self.imgui_ui.get_background_draw_list();
        let n_idx = dl.idx_buf.len();
        let n_vtx = dl.vtx_buf.len();
        for cmd in &dl.cmd_buf {
            if cmd.elem_count == 0 { continue; }
            let mut i = 0usize;
            while i + 2 < cmd.elem_count as usize {
                let b = cmd.idx_offset as usize + i;
                if b + 2 >= n_idx { break; }
                let vi0 = dl.idx_buf[b  ] as usize + cmd.vtx_offset as usize;
                let vi1 = dl.idx_buf[b+1] as usize + cmd.vtx_offset as usize;
                let vi2 = dl.idx_buf[b+2] as usize + cmd.vtx_offset as usize;
                if vi0 >= n_vtx || vi1 >= n_vtx || vi2 >= n_vtx { i += 3; continue; }
                let v0 = dl.vtx_buf[vi0]; let v1 = dl.vtx_buf[vi1]; let v2 = dl.vtx_buf[vi2];
                bg.add_triangle(v0.pos, v1.pos, v2.pos, unpack_col(v0.col)).filled(true).build();
                i += 3;
            }
        }
    }

    /// Access the underlying imgui `Ui` directly for native imgui rendering.
    /// This is the recommended way to draw widgets — the imruski-core triangle
    /// path has no proper font atlas integration so text is invisible.
    pub fn imgui(&mut self) -> &mut hudhook::imgui::Ui {
        self.imgui_ui
    }

    /// Current render-target size in pixels.
    pub fn display_size(&self) -> Vec2 {
        self.display_size
    }
}

/// Unpack a packed 0xAABBGGRR colour into an RGBA f32 array.
#[inline]
fn unpack_col(c: u32) -> [f32; 4] {
    [
        (c & 0xFF)          as f32 / 255.0,
        ((c >> 8)  & 0xFF)  as f32 / 255.0,
        ((c >> 16) & 0xFF)  as f32 / 255.0,
        ((c >> 24) & 0xFF)  as f32 / 255.0,
    ]
}

//  ImRuskiRenderLoop trait 

/// Implement this trait and hand it to [`Dx11Hook::new`].
pub trait ImRuskiRenderLoop: Send + 'static {
    /// Called once on the render thread before the first frame, with the
    /// imgui Context — use this to apply global style/font settings.
    fn on_init(&mut self, _ctx: &mut hudhook::imgui::Context) {}
    /// Called every frame before `IDXGISwapChain::Present`.
    fn render(&mut self, frame: &mut Dx11Frame<'_>);
}

//  HudhookBridge 

struct HudhookBridge {
    inner: Arc<Mutex<Box<dyn ImRuskiRenderLoop>>>,
}

impl hudhook::ImguiRenderLoop for HudhookBridge {
    /// Called once on the render thread inside `Pipeline::new()`, before
    /// the WndProc is subclassed and before `render()` is ever called.
    /// This is the EARLIEST point on the render thread we can act.
    fn initialize<'a>(
        &'a mut self,
        ctx: &mut hudhook::imgui::Context,
        _render_context: &'a mut dyn hudhook::RenderContext,
    ) {
        dx11_log("initialize: enter (render thread)");
        unsafe { ensure_thread_init(); }
        if let Ok(mut rl) = self.inner.lock() {
            rl.on_init(ctx);
        }
        dx11_log("initialize: thread-init OK");
    }

    /// Called every frame just before `render()`.
    fn before_render<'a>(
        &'a mut self,
        _ctx: &mut hudhook::imgui::Context,
        _render_context: &'a mut dyn hudhook::RenderContext,
    ) {
        // ensure_thread_init is already called in initialize(); this is a
        // cheap guard for any other threads that might reach here.
        unsafe { ensure_thread_init(); }
    }

    fn render(&mut self, ui: &mut hudhook::imgui::Ui) {
        // Belt-and-suspenders: also call here in case initialize() was missed.
        unsafe { ensure_thread_init(); }

        // Limit verbose logging to first 5 frames to avoid hammering the disk.
        static RENDER_COUNT: AtomicU32 = AtomicU32::new(0);
        let frame_n = RENDER_COUNT.fetch_add(1, Ordering::Relaxed);
        let verbose = frame_n < 5;

        if verbose { dx11_log("render: enter"); }

        let display_size = {
            let io = ui.io();
            Vec2::new(io.display_size[0], io.display_size[1])
        };
        if display_size.x < 1.0 || display_size.y < 1.0 {
            if verbose { dx11_log("render: skip (display size < 1)"); }
            return;
        }
        let mut frame = Dx11Frame {
            imgui_ui:     ui,
            font:         Dx11FontAtlas,
            display_size,
        };
        if verbose { dx11_log("render: locking inner..."); }
        if let Ok(mut rl) = self.inner.lock() {
            if verbose { dx11_log("render: lock OK, calling user render..."); }
            rl.render(&mut frame);
            if verbose { dx11_log("render: user render done"); }
        } else {
            dx11_log("render: lock FAILED (poisoned)");
        }
        if verbose { dx11_log("render: exit"); }
    }

    /// Called on the render thread when a queued WndProc message is processed.
    fn on_wnd_proc(
        &self,
        _hwnd: hudhook::windows::Win32::Foundation::HWND,
        _umsg: u32,
        _wparam: hudhook::windows::Win32::Foundation::WPARAM,
        _lparam: hudhook::windows::Win32::Foundation::LPARAM,
    ) {
        // on_wnd_proc re-enters on the render thread (not the WndProc thread),
        // so CRT init from initialize() already covers this. No-op otherwise.
    }
}

//  Dx11Hook  public entry point 

/// Hook `IDXGISwapChain::Present` and drive an [`ImRuskiRenderLoop`].
pub struct Dx11Hook {
    render_loop: Arc<Mutex<Box<dyn ImRuskiRenderLoop>>>,
}

impl Dx11Hook {
    /// `dll_base` — remote base address of the manually-mapped payload DLL.
    /// `dll_ep`   — absolute VA of `_DllMainCRTStartup` (the PE entry point).
    pub fn new(render_loop: Box<dyn ImRuskiRenderLoop>, dll_base: usize, dll_ep: usize) -> Self {
        DLL_BASE.store(dll_base, Ordering::Relaxed);
        DLL_EP.store(dll_ep,   Ordering::Relaxed);
        Self { render_loop: Arc::new(Mutex::new(render_loop)) }
    }

    /// Install the DX11 hook (game uses DX11 primary renderer).
    pub fn install(self) -> Result<(), Dx11Error> {
        use hudhook::hooks::dx11::ImguiDx11Hooks;
        use hudhook::Hudhook;

        dx11_log("install: calling Hudhook builder...");
        let bridge = HudhookBridge { inner: self.render_loop };
        dx11_log("install: with::<ImguiDx11Hooks>...");
        let builder = Hudhook::builder().with::<ImguiDx11Hooks>(bridge);
        dx11_log("install: build()...");
        let built = builder.build();
        dx11_log("install: apply()...");
        let result = built.apply();
        match &result {
            Ok(_)  => dx11_log("install: apply OK"),
            Err(e) => dx11_log(&format!("install: apply ERR: {e:?}")),
        }
        result.map_err(|e| Dx11Error::HookInstall(format!("{e:?}")))
    }
}

// ─── Module-level probe (for diagnostics) ───────────────────────────────────
//
// Reads the base address of dxgi.dll and d3d12.dll from the process module list
// using GetModuleHandleW and logs these so we can compare against what minhook
// patches. Zero COM objects created — safe to call from any thread at any time.

unsafe fn probe_dxgi_module() {
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::core::PCWSTR;

    // Static wide string literals — no heap allocation.
    const DXGI: &[u16]    = &[b'd' as u16, b'x' as u16, b'g' as u16, b'i' as u16,
                               b'.' as u16, b'd' as u16, b'l' as u16, b'l' as u16, 0];
    const D3D12: &[u16]   = &[b'd' as u16, b'3' as u16, b'd' as u16, b'1' as u16, b'2' as u16,
                               b'.' as u16, b'd' as u16, b'l' as u16, b'l' as u16, 0];
    const D3DCORE: &[u16] = &[b'D' as u16, b'3' as u16, b'D' as u16, b'1' as u16, b'2' as u16,
                               b'C' as u16, b'o' as u16, b'r' as u16, b'e' as u16,
                               b'.' as u16, b'd' as u16, b'l' as u16, b'l' as u16, 0];

    dx11_log("probe: calling GetModuleHandleW for dxgi.dll");
    let dxgi_h    = GetModuleHandleW(PCWSTR(DXGI.as_ptr()));
    let dxgi_base = dxgi_h.map(|h| h.0 as usize).unwrap_or(0);
    dx11_log(&format!("probe: dxgi.dll      base = {dxgi_base:#018x}"));

    let d3d12_h    = GetModuleHandleW(PCWSTR(D3D12.as_ptr()));
    let d3d12_base = d3d12_h.map(|h| h.0 as usize).unwrap_or(0);
    dx11_log(&format!("probe: d3d12.dll     base = {d3d12_base:#018x}"));

    let core_h    = GetModuleHandleW(PCWSTR(D3DCORE.as_ptr()));
    let core_base = core_h.map(|h| h.0 as usize).unwrap_or(0);
    dx11_log(&format!("probe: D3D12Core.dll base = {core_base:#018x}"));
}

pub use imruski_core as core;
