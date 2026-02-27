//! ImRuski Payload DLL
//!
//! Inject into any DX11 game and render an Apple-style overlay using
//! **imruski-core** via **imruski-dx11** (hudhook Present hook).
//!
//! Press INSERT to toggle visibility.

#![cfg(windows)]
#![allow(non_snake_case)]

use imruski_dx11::{Dx11Frame, Dx11Hook, ImRuskiRenderLoop};

// --- Exception-table registration ---------------------------------------------
// x64 Windows requires every manually-mapped code region to register its
// .pdata (RUNTIME_FUNCTION) table via RtlAddFunctionTable so that the OS
// stack-unwinder can walk frames through the DLL.  Without this, the very
// first Rust unwind (error propagation, panic, or any SEH) triggers an
// unhandled EXCEPTION_NONCONTINUABLE and kills the host process.

#[link(name = "ntdll")]
extern "system" {
    fn RtlAddFunctionTable(
        function_table: *const u8,
        entry_count:    u32,
        base_address:   u64,
    ) -> u8;
}

unsafe fn register_exception_table(base: usize) {
    // Validate MZ magic
    if *(base as *const u16) != 0x5A4D { return; }
    let lfanew = *((base + 60) as *const u32) as usize;
    // Validate PE signature
    if *((base + lfanew) as *const u32) != 0x0000_4550 { return; }
    // DataDirectory[3] = exception directory.
    // OptionalHeader64 starts at lfanew + 24 (4-byte sig + 20-byte FileHeader).
    // DataDirectory begins at offset 112 inside OptionalHeader64.
    // Entry 3 is at +24 from DataDirectory start  (3 * 8).
    let excep = base + lfanew + 24 + 112 + 24;
    let rva   = *(excep       as *const u32);
    let size  = *((excep + 4) as *const u32);
    if rva == 0 || size < 12 { return; }
    RtlAddFunctionTable(
        (base + rva as usize) as *const u8,
        size / 12,  // each RUNTIME_FUNCTION entry is 12 bytes
        base as u64,
    );
}

// ── Dark glass theme ──────────────────────────────────────────────────────────

fn apply_dark_theme(ctx: &mut imruski_dx11::imgui::Context) {
    let s = ctx.style_mut();
    s.window_rounding    = 16.0;
    s.child_rounding     = 10.0;
    s.frame_rounding     = 8.0;
    s.scrollbar_rounding = 8.0;
    s.grab_rounding      = 8.0;
    s.tab_rounding       = 6.0;
    s.window_border_size = 1.5;
    s.frame_border_size  = 0.0;
    s.window_padding     = [16.0, 14.0];
    s.frame_padding      = [10.0,  6.0];
    s.item_spacing       = [ 8.0,  8.0];
    s.indent_spacing     = 18.0;
    s.window_title_align = [0.5, 0.5];   // center the title text

    use imruski_dx11::imgui::StyleColor as SC;
    // Deep dark glass
    s[SC::WindowBg]          = [0.07, 0.07, 0.11, 0.95];
    s[SC::ChildBg]           = [0.10, 0.10, 0.16, 0.80];
    s[SC::PopupBg]           = [0.07, 0.07, 0.11, 0.95];
    // Glowing purple border
    s[SC::Border]            = [0.48, 0.22, 0.88, 0.72];
    s[SC::BorderShadow]      = [0.00, 0.00, 0.00, 0.00];
    // Deep purple title bar
    s[SC::TitleBg]           = [0.10, 0.06, 0.20, 1.00];
    s[SC::TitleBgActive]     = [0.15, 0.08, 0.30, 1.00];
    s[SC::TitleBgCollapsed]  = [0.08, 0.04, 0.14, 0.85];
    // Text
    s[SC::Text]              = [0.92, 0.90, 1.00, 1.00];
    s[SC::TextDisabled]      = [0.42, 0.40, 0.52, 1.00];
    // Frames / inputs
    s[SC::FrameBg]           = [0.18, 0.15, 0.30, 0.80];
    s[SC::FrameBgHovered]    = [0.26, 0.22, 0.40, 0.90];
    s[SC::FrameBgActive]     = [0.32, 0.26, 0.50, 1.00];
    // Buttons — vivid purple accent
    s[SC::Button]            = [0.38, 0.16, 0.78, 0.88];
    s[SC::ButtonHovered]     = [0.52, 0.26, 0.96, 1.00];
    s[SC::ButtonActive]      = [0.28, 0.10, 0.60, 1.00];
    // Controls
    s[SC::CheckMark]         = [0.70, 0.45, 1.00, 1.00];
    s[SC::SliderGrab]        = [0.55, 0.28, 0.98, 1.00];
    s[SC::SliderGrabActive]  = [0.68, 0.42, 1.00, 1.00];
    // Header
    s[SC::Header]            = [0.35, 0.15, 0.68, 0.55];
    s[SC::HeaderHovered]     = [0.48, 0.24, 0.85, 0.80];
    s[SC::HeaderActive]      = [0.55, 0.28, 0.92, 1.00];
    // Separator
    s[SC::Separator]         = [0.35, 0.18, 0.60, 0.70];
    s[SC::SeparatorHovered]  = [0.52, 0.28, 0.82, 1.00];
    s[SC::SeparatorActive]   = [0.62, 0.38, 0.94, 1.00];
    // Scrollbar
    s[SC::ScrollbarBg]       = [0.08, 0.08, 0.12, 0.60];
    s[SC::ScrollbarGrab]     = [0.38, 0.16, 0.72, 0.80];
    s[SC::ScrollbarGrabHovered] = [0.52, 0.26, 0.88, 1.00];
    s[SC::ScrollbarGrabActive]  = [0.62, 0.38, 0.98, 1.00];
    // Resize grip
    s[SC::ResizeGrip]        = [0.48, 0.22, 0.88, 0.30];
    s[SC::ResizeGripHovered] = [0.58, 0.32, 0.96, 0.80];
    s[SC::ResizeGripActive]  = [0.68, 0.44, 1.00, 1.00];
    // Tabs
    s[SC::Tab]               = [0.14, 0.08, 0.26, 0.90];
    s[SC::TabHovered]        = [0.48, 0.24, 0.85, 1.00];
    s[SC::TabActive]         = [0.36, 0.16, 0.70, 1.00];
    s[SC::TabUnfocused]      = [0.10, 0.06, 0.18, 0.90];
    s[SC::TabUnfocusedActive]= [0.28, 0.12, 0.55, 1.00];
}

// ── WIN32: raw key read (user32 is already in the IAT via hudhook) ─────────────

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(vKey: i32) -> i16;
}

// ── Overlay state ─────────────────────────────────────────────────────────────

struct ImRuskiOverlay {
    // visibility + animation
    visible:     bool,
    anim_t:      f32,    // 0.0 = fully hidden  →  1.0 = fully shown
    key_held:    bool,   // INSERT debounce
    // UI
    active_tab:  usize,
    // Combat
    aimbot:      bool,
    aim_fov:     f32,
    aim_smooth:  f32,
    no_recoil:   bool,
    rapid_fire:  bool,
    // Visual
    esp_box:     bool,
    esp_health:  bool,
    esp_name:    bool,
    glow:        bool,
    // Misc
    speed:       f32,
    inf_ammo:    bool,
    no_spread:   bool,
    themed:      bool,
}

impl ImRuskiOverlay {
    fn new() -> Self {
        Self {
            visible:    true,
            anim_t:     0.0,
            key_held:   false,
            active_tab: 0,
            aimbot:     false,
            aim_fov:    65.0,
            aim_smooth: 3.0,
            no_recoil:  false,
            rapid_fire: false,
            esp_box:    false,
            esp_health: false,
            esp_name:   false,
            glow:       false,
            speed:      1.0,
            inf_ammo:   false,
            no_spread:  false,
            themed:     false,
        }
    }
}

// ── Render loop ───────────────────────────────────────────────────────────────

impl ImRuskiRenderLoop for ImRuskiOverlay {
    fn on_init(&mut self, ctx: &mut imruski_dx11::imgui::Context) {
        apply_dark_theme(ctx);
        self.themed = true;
    }

    fn render(&mut self, frame: &mut Dx11Frame<'_>) {
        // ── INSERT key toggle (debounced) ─────────────────────────────────
        let ins = unsafe { (GetAsyncKeyState(0x2D) as u16 & 0x8000) != 0 };
        if ins && !self.key_held {
            self.visible  = !self.visible;
            self.key_held = true;
        } else if !ins {
            self.key_held = false;
        }

        // ── Exponential-ease animation  (≈250 ms at 60 fps) ──────────────
        let target = if self.visible { 1.0f32 } else { 0.0 };
        self.anim_t += (target - self.anim_t) * 0.18;
        if self.anim_t < 0.005 { self.anim_t = 0.0; }
        if self.anim_t > 0.995 { self.anim_t = 1.0; }
        if self.anim_t < 0.01  { return; }

        let ui = frame.imgui();

        // Fade entire window with the animation progress
        let _alpha = ui.push_style_var(imruski_dx11::imgui::StyleVar::Alpha(self.anim_t));

        // ── Locals — avoids borrow conflicts inside the closure ───────────
        let mut open       = self.visible;
        let mut tab        = self.active_tab;
        let mut aimbot     = self.aimbot;
        let mut aim_fov    = self.aim_fov;
        let mut aim_smooth = self.aim_smooth;
        let mut no_recoil  = self.no_recoil;
        let mut rapid_fire = self.rapid_fire;
        let mut esp_box    = self.esp_box;
        let mut esp_health = self.esp_health;
        let mut esp_name   = self.esp_name;
        let mut glow       = self.glow;
        let mut speed      = self.speed;
        let mut inf_ammo   = self.inf_ammo;
        let mut no_spread  = self.no_spread;
        let mut close_req  = false;

        ui.window("[~] ImRuski  v1.0")
            .size([390.0, 0.0], imruski_dx11::imgui::Condition::FirstUseEver)
            .build(|| {
                use imruski_dx11::imgui::StyleColor as SC;

                // ── Tab bar ───────────────────────────────────────────────
                let tab_labels = ["  Combat  ", "  Visual  ", "  Misc  ", "  About  "];
                for (i, &label) in tab_labels.iter().enumerate() {
                    if i > 0 { ui.same_line(); }
                    let active = tab == i;
                    let (bc, bh, ba) = if active {
                        ([0.48f32, 0.22, 0.92, 1.00],
                         [0.60,    0.34, 1.00, 1.00],
                         [0.36,    0.14, 0.72, 1.00])
                    } else {
                        ([0.17f32, 0.14, 0.27, 0.88],
                         [0.28,    0.22, 0.44, 1.00],
                         [0.12,    0.10, 0.20, 1.00])
                    };
                    let _c1 = ui.push_style_color(SC::Button,       bc);
                    let _c2 = ui.push_style_color(SC::ButtonHovered, bh);
                    let _c3 = ui.push_style_color(SC::ButtonActive,  ba);
                    if ui.button_with_size(label, [84.0, 26.0]) { tab = i; }
                }

                ui.spacing();
                ui.separator();
                ui.spacing();

                // ── Tab content ───────────────────────────────────────────
                match tab {
                    // ── COMBAT ───────────────────────────────────────────
                    0 => {
                        ui.text("  > Aimbot");
                        ui.spacing();
                        ui.checkbox("Enable Aimbot", &mut aimbot);
                        if aimbot {
                            ui.slider("  FOV",    10.0f32, 180.0, &mut aim_fov);
                            ui.slider("  Smooth",  1.0f32,  10.0, &mut aim_smooth);
                        }
                        ui.spacing();
                        ui.separator();
                        ui.spacing();
                        ui.text("  > Shooting");
                        ui.spacing();
                        ui.checkbox("No Recoil",  &mut no_recoil);
                        ui.checkbox("Rapid Fire", &mut rapid_fire);
                    }

                    // ── VISUAL ───────────────────────────────────────────
                    1 => {
                        ui.text("  > ESP");
                        ui.spacing();
                        ui.checkbox("Box ESP",      &mut esp_box);
                        ui.checkbox("Health Bar",   &mut esp_health);
                        ui.checkbox("Name Tag",     &mut esp_name);
                        ui.checkbox("Player Glow",  &mut glow);
                    }

                    // ── MISC ─────────────────────────────────────────────
                    2 => {
                        ui.text("  > Movement");
                        ui.spacing();
                        ui.slider("Speed Mult", 1.0f32, 3.0, &mut speed);
                        ui.spacing();
                        ui.separator();
                        ui.spacing();
                        ui.text("  > Other");
                        ui.spacing();
                        ui.checkbox("Infinite Ammo", &mut inf_ammo);
                        ui.checkbox("No Spread",      &mut no_spread);
                    }

                    // ── ABOUT ────────────────────────────────────────────
                    _ => {
                        ui.spacing();
                        ui.text("  [~] ImRuski  v1.0");
                        ui.spacing();
                        ui.text_disabled("  DX11 overlay  |  hudhook + imgui-rs");
                        ui.spacing();
                        ui.separator();
                        ui.spacing();
                        ui.text("  [INS]  Toggle overlay");
                        ui.text("  Drag title bar to move");
                        ui.text("  Resize from bottom-right corner");
                        ui.spacing();
                    }
                }

                ui.spacing();
                ui.separator();
                ui.spacing();

                // ── Centered red close button ─────────────────────────────
                let btn_w  = 90.0f32;
                let avail  = ui.content_region_avail()[0];
                let cur    = ui.cursor_pos();
                let offset = ((avail - btn_w) * 0.5).max(0.0);
                ui.set_cursor_pos([cur[0] + offset, cur[1]]);

                let _r1 = ui.push_style_color(SC::Button,        [0.65, 0.12, 0.12, 1.0]);
                let _r2 = ui.push_style_color(SC::ButtonHovered,  [0.86, 0.22, 0.22, 1.0]);
                let _r3 = ui.push_style_color(SC::ButtonActive,   [0.48, 0.08, 0.08, 1.0]);
                let _r4 = ui.push_style_color(SC::Text,           [1.00, 1.00, 1.00, 1.00]);
                if ui.button_with_size("  Close  ", [btn_w, 0.0]) { close_req = true; }
            });

        // Write locals back to self
        if close_req { open = false; }
        self.visible    = open;
        self.active_tab = tab;
        self.aimbot     = aimbot;
        self.aim_fov    = aim_fov;
        self.aim_smooth = aim_smooth;
        self.no_recoil  = no_recoil;
        self.rapid_fire = rapid_fire;
        self.esp_box    = esp_box;
        self.esp_health = esp_health;
        self.esp_name   = esp_name;
        self.glow       = glow;
        self.speed      = speed;
        self.inf_ammo   = inf_ammo;
        self.no_spread  = no_spread;
    }
}

//  Entry points 

/// Manual-map injector entry point.
/// Called from shellcode BEFORE the CRT is initialised.
/// Must use only Win32 API calls (no std, no TLS).
/// Steps:
///   1. Allocate a real TLS slot and patch __tls_index in our .data so the
///      CRT doesn't clobber TLS slot 0 (which belongs to ntdll/kernel32).
///   2. Call _DllMainCRTStartup(base, DLL_PROCESS_ATTACH, NULL) � this
///      initialises the security cookie, runs global ctors, and calls DllMain
///      which in turn spawns the hook thread via payload_attach.
#[no_mangle]
pub extern "C" fn imruski_init(base: usize) {
    unsafe {
        // Validate MZ / PE signatures
        if *(base as *const u16) != 0x5A4D { return; }
        let lfanew = *((base + 60) as *const u32) as usize;
        if *((base + lfanew) as *const u32) != 0x0000_4550 { return; }

        // -- 1. Patch TLS index ----------------------------------------------
        // OptionalHeader64 starts at lfanew+24.
        // DataDirectory[9] (TLS) is at OPTHDR+112 + 9*8 = OPTHDR+184.
        let dd_tls = base + lfanew + 24 + 112 + 72; // DataDirectory[9].VirtualAddress
        let tls_rva = *(dd_tls as *const u32) as usize;
        if tls_rva != 0 {
            // IMAGE_TLS_DIRECTORY64:
            //  +0  StartAddressOfRawData (VA, u64)
            //  +8  EndAddressOfRawData   (VA, u64)
            //  +16 AddressOfIndex        (VA pointing to __tls_index, u64)
            let td = base + tls_rva;
            let addr_of_index = *((td + 16) as *const u64) as usize;
            if addr_of_index != 0 {
                let slot = TlsAlloc();
                if slot != 0xFFFF_FFFF {
                    // Stamp the real slot number into __tls_index
                    *(addr_of_index as *mut u32) = slot;
                    payload_log("imruski_init: TLS slot patched");
                } else {
                    payload_log("imruski_init: TlsAlloc FAILED");
                }
            }
        } else {
            payload_log("imruski_init: no TLS directory");
        }

        // -- 1b. Fix IAT in-process -------------------------------------------
        // Non-KnownDLLs (VCRUNTIME140, d3d11, d3dcompiler_47, etc.) may load at
        // different base addresses in the game vs. the injector.  Re-resolve
        // every IAT slot here, before the CRT ever runs.
        fix_iat(base);
        payload_log("imruski_init: IAT fixed");

        // -- 2. CRT startup --------------------------------------------------
        // AddressOfEntryPoint is at OPTHDR+16 ? absolute offset lfanew+24+16.
        let ep_rva = *((base + lfanew + 40) as *const u32) as usize;
        if ep_rva != 0 {
            let crt: unsafe extern "system" fn(usize, u32, usize) -> i32 =
                core::mem::transmute(base + ep_rva);
            payload_log("imruski_init: calling CRT startup...");
            crt(base, 1 /* DLL_PROCESS_ATTACH */, 0);
            payload_log("imruski_init: CRT startup returned");
        }
    }
}

/// A pointer to `imruski_init` placed in a dedicated `.imrski` section.
/// After the injector applies relocations, this 8-byte slot holds the
/// absolute VA of `imruski_init` in the remote process.  The injector
/// reads this slot (section_rva + remote_base), treats it as a function
/// pointer, and calls it � bypassing the export table entirely.
#[link_section = ".imrski"]
#[used]
static IMRUSKI_INIT_PTR: unsafe extern "C" fn(usize) = imruski_init;

/// Standard Windows DLL entry point � called by `_DllMainCRTStartup` when the
/// DLL is loaded normally (not manually mapped).
#[no_mangle]
pub extern "system" fn DllMain(
    module:      *mut core::ffi::c_void,
    call_reason: u32,
    _reserved:   *mut core::ffi::c_void,
) -> bool {
    if call_reason == 1 /* DLL_PROCESS_ATTACH */ {
        payload_attach(module);
    }
    true
}

// Raw Win32 logging � no Rust std file I/O, no heap allocation for the path.
// Works even when the CRT / Rust std isn't fully initialised for this module.
fn payload_log(msg: &str) {
    #[allow(non_snake_case)]
    extern "system" {
        fn CreateFileA(
            p: *const u8, access: u32, share: u32,
            sa: *const core::ffi::c_void, disp: u32, flags: u32,
            tmpl: *const core::ffi::c_void,
        ) -> isize;
        fn SetFilePointer(h: isize, dist: i32, hi: *mut i32, method: u32) -> u32;
        fn WriteFile(h: isize, buf: *const u8, n: u32, written: *mut u32,
                     ov: *const core::ffi::c_void) -> i32;
        fn CloseHandle(h: isize) -> i32;
    }
    static PATH: &[u8] = b"C:\\Users\\mouss\\Music\\payload_log.txt\0";
    static PREFIX: &[u8] = b"[PAYLOAD] ";
    static NL: &[u8] = b"\r\n";
    unsafe {
        let h = CreateFileA(PATH.as_ptr(), 0x4000_0000 /*GENERIC_WRITE*/,
                            3 /*SHARE_RW*/, core::ptr::null(),
                            4 /*OPEN_ALWAYS*/, 0x80 /*NORMAL*/, core::ptr::null());
        if h == -1 { return; }
        SetFilePointer(h, 0, core::ptr::null_mut(), 2 /*FILE_END*/);
        let mut w = 0u32;
        WriteFile(h, PREFIX.as_ptr(), PREFIX.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, msg.as_ptr(), msg.len() as u32, &mut w, core::ptr::null());
        WriteFile(h, NL.as_ptr(), NL.len() as u32, &mut w, core::ptr::null());
        CloseHandle(h);
    }
}

fn payload_attach(module: *mut core::ffi::c_void) {
    // 1. Register .pdata so Rust stack-unwinding is safe in this mapped DLL.
    unsafe { register_exception_table(module as usize); }
    payload_log("payload_attach called � exception table registered");

    // 2. Spawn via raw CreateThread � Rust std::thread::spawn touches TLS
    //    internals that are never initialised for a manually-mapped DLL and
    //    will crash instantly.  A raw Win32 thread has no such requirement.
    //    The thread proc is a plain extern "system" fn with no Rust std TLS.
    unsafe {
        // Box the base address so the thread owns it.
        let param = Box::into_raw(Box::new(module as usize)) as *mut core::ffi::c_void;
        let h = CreateThread(core::ptr::null(), 8 * 1024 * 1024 /* 8 MB reserved */, Some(hook_thread), param, 0x00010000 /* STACK_SIZE_PARAM_IS_A_RESERVATION */, core::ptr::null_mut());
        if h.is_null() {
            payload_log("CreateThread FAILED");
        } else {
            payload_log("CreateThread OK � hook_thread spawned");
            CloseHandle(h);
        }
    }
    payload_log("payload_attach returning");
}

#[link(name = "kernel32")]
extern "system" {
    fn CreateThread(
        lpThreadAttributes: *const core::ffi::c_void,
        dwStackSize: usize,
        lpStartAddress: Option<unsafe extern "system" fn(*mut core::ffi::c_void) -> u32>,
        lpParameter: *mut core::ffi::c_void,
        dwCreationFlags: u32,
        lpThreadId: *mut u32,
    ) -> *mut core::ffi::c_void;
    fn CloseHandle(hObject: *mut core::ffi::c_void) -> i32;
    fn Sleep(dwMilliseconds: u32);
    fn TlsAlloc() -> u32;
    // Used by fix_iat � kernel32 is a KnownDLL so these addresses are valid in any process.
    fn GetModuleHandleA(lpModuleName: *const u8) -> usize;
    fn LoadLibraryA(lpLibFileName: *const u8) -> usize;
    fn GetProcAddress(hModule: usize, lpProcName: *const u8) -> usize;
}

// --- In-process IAT fixer ------------------------------------------------------
//
// The injector resolves imports from its *own* address space.  Non-KnownDLLs
// (VCRUNTIME140, d3d11, d3dcompiler_47, �) can load at different bases in the
// game process, so those IAT entries end up pointing into data pages ? DEP
// fault the moment imgui calls memset/memcpy.
//
// `fix_iat` runs in the game process (inside imruski_init, before CRT startup)
// and re-resolves every IAT slot with the game's own LoadLibraryA /
// GetProcAddress.  Only kernel32.dll calls are used here (kernel32 IS a
// KnownDLL � same VA in all processes on the same boot).
unsafe fn fix_iat(base: usize) {
    // Validate MZ + PE signatures
    if *(base as *const u16) != 0x5A4D { return; }
    let lfanew = *((base + 60) as *const u32) as usize;
    if *((base + lfanew) as *const u32) != 0x0000_4550 { return; }

    // OptionalHeader64 starts at lfanew + 4 (signature) + 20 (FileHeader) = lfanew + 24.
    // DataDirectory[1] (import) = OptHdr + 112 + 1*8 = OptHdr + 120.
    let opt = base + lfanew + 24;
    let import_rva  = *((opt + 120) as *const u32) as usize;
    let import_size = *((opt + 124) as *const u32) as usize;
    if import_rva == 0 || import_size == 0 { return; }

    // Walk IMAGE_IMPORT_DESCRIPTOR table (20 bytes each).
    let mut desc = base + import_rva;
    loop {
        //  +0  OriginalFirstThunk (u32)
        //  +4  TimeDateStamp      (u32)
        //  +8  ForwarderChain     (u32)
        // +12  Name               (u32)  ? RVA of DLL name string
        // +16  FirstThunk         (u32)  ? RVA of IAT
        let oft_rva      = *((desc +  0) as *const u32) as usize;
        let dll_name_rva = *((desc + 12) as *const u32) as usize;
        let iat_rva      = *((desc + 16) as *const u32) as usize;

        if dll_name_rva == 0 { break; } // end sentinel

        let dll_name = (base + dll_name_rva) as *const u8;
        let mut hmod = GetModuleHandleA(dll_name);
        if hmod == 0 { hmod = LoadLibraryA(dll_name); }

        if hmod != 0 {
            // Use OriginalFirstThunk (if present) as lookup source, IAT as write target.
            let thunk_src = if oft_rva != 0 { base + oft_rva } else { base + iat_rva };
            let iat       = base + iat_rva;

            let mut i = 0usize;
            loop {
                let thunk = *((thunk_src + i * 8) as *const u64);
                if thunk == 0 { break; }

                let addr = if thunk & 0x8000_0000_0000_0000 != 0 {
                    // Import by ordinal
                    GetProcAddress(hmod, (thunk & 0xFFFF) as usize as *const u8)
                } else {
                    // Import by name: IMAGE_IMPORT_BY_NAME = 2-byte Hint + name
                    GetProcAddress(hmod, (base + thunk as usize + 2) as *const u8)
                };

                if addr != 0 {
                    *((iat + i * 8) as *mut u64) = addr as u64;
                }
                i += 1;
            }
        }

        desc += 20; // next descriptor
    }
}

unsafe extern "system" fn hook_thread(param: *mut core::ffi::c_void) -> u32 {
    let base = *(param as *mut usize);
    drop(Box::from_raw(param as *mut usize));

    payload_log("hook_thread: started");

    // The OS never calls DLL_THREAD_ATTACH for a manually-mapped DLL.
    // Without it, the MSVC CRT never initialises per-thread state (security
    // cookie, errno, Rust std thread-locals) on this new thread.
    // Fix: call _DllMainCRTStartup(base, DLL_THREAD_ATTACH=2, NULL) ourselves.
    {
        // Find the PE entry point (AddressOfEntryPoint) from our own headers.
        let ep = {
            let dos = base as *const u16;
            if *dos == 0x5A4D {
                let lfanew = *((base + 60) as *const u32) as usize;
                if *((base + lfanew) as *const u32) == 0x0000_4550 {
                    // OptionalHeader64 starts at lfanew+24; AddressOfEntryPoint is at +16
                    let ep_rva = *((base + lfanew + 24 + 16) as *const u32) as usize;
                    if ep_rva != 0 { base + ep_rva } else { 0 }
                } else { 0 }
            } else { 0 }
        };
        if ep != 0 {
            let crt_start: unsafe extern "system" fn(*mut core::ffi::c_void, u32, *mut core::ffi::c_void) -> i32 =
                core::mem::transmute(ep);
            crt_start(base as *mut core::ffi::c_void, 2 /* DLL_THREAD_ATTACH */, core::ptr::null_mut());
            payload_log("hook_thread: DLL_THREAD_ATTACH OK");
        } else {
            payload_log("hook_thread: WARNING no PE entry found");
        }
    }

    payload_log("hook_thread: sleeping 3s...");
    Sleep(3000);
    payload_log("hook_thread: sleep done");

    let overlay: Box<dyn ImRuskiRenderLoop> = Box::new(ImRuskiOverlay::new());
    payload_log("hook_thread: overlay boxed OK");

    payload_log("hook_thread: Dx11Hook::new...");
    // Read the PE entry point so the dx11 backend can call DLL_THREAD_ATTACH
    // on the game's render thread before hudhook's tracing fires.
    let dll_ep = unsafe {
        let lfanew = *((base + 60) as *const u32) as usize;
        let ep_rva = *((base + lfanew + 24 + 16) as *const u32) as usize;
        if ep_rva != 0 { base + ep_rva } else { 0 }
    };
    let hook = Dx11Hook::new(overlay, base, dll_ep);
    payload_log("hook_thread: Dx11Hook::new OK");

    payload_log("hook_thread: install...");
    match hook.install() {
        Ok(_)  => payload_log("hook_thread: install OK � overlay LIVE"),
        Err(e) => payload_log(&format!("hook_thread: install Err: {e:?}")),
    }
    0
}
