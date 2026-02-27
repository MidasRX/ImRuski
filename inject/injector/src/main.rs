//! # ImRuski Manual-Map Injector
//!
//! Injects a DLL into a running DX11 process without using LoadLibrary.
//! Steps:
//!   1. Find process by name
//!   2. Parse x64 PE headers (custom structs, no external PE crate)
//!   3. Allocate RWX memory in target
//!   4. Copy headers + sections
//!   5. Fix base relocations
//!   6. Resolve import address table
//!   7. Run shellcode that calls DllMain then ExitThread

// ---------------------------------------------------------------------------
// Logger: writes every line to both stdout AND a file
// ---------------------------------------------------------------------------
const LOG_PATH: &str = r"C:\Users\mouss\Music\new cs2.txt";

struct Log {
    file: std::fs::File,
}

impl Log {
    fn open() -> Self {
        use std::fs::OpenOptions;
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(LOG_PATH)
            .unwrap_or_else(|e| panic!("Cannot open log file {LOG_PATH}: {e}"));
        Log { file }
    }

    fn raw(&mut self, msg: &str) {
        use std::io::Write;
        println!("{msg}");
        let _ = writeln!(self.file, "{msg}");
        let _ = self.file.flush();
    }
}

macro_rules! log {
    ($l:expr, $($arg:tt)*) => {
        $l.raw(&format!($($arg)*))
    };
}

fn main() {
    #[cfg(windows)]
    { if let Err(e) = run() { eprintln!("[!] Error: {e:#}"); std::process::exit(1); } }
    #[cfg(not(windows))]
    { eprintln!("imruski-injector is Windows-only."); }
}

// ---------------------------------------------------------------------------
// Minimal PE type definitions (avoids windows-crate feature fragility)
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod pe {
    pub const MZ_MAGIC: u16 = 0x5A4D;
    pub const PE_SIG:   u32 = 0x0000_4550;
    pub const MACHINE_AMD64: u16 = 0x8664;
    pub const DIR_IMPORT:    usize = 1;
    pub const DIR_BASERELOC: usize = 5;
    pub const DIR_TLS:        usize = 9;
    pub const IMAGE_ORDINAL_FLAG64: u64 = 0x8000_0000_0000_0000;

    #[repr(C, packed)] pub struct DosHeader     { pub e_magic: u16, _pad: [u8;58], pub e_lfanew: i32 }
    #[repr(C)] pub struct DataDirectory          { pub virtual_address: u32, pub size: u32 }
    #[repr(C)] pub struct BaseRelocation         { pub virtual_address: u32, pub size_of_block: u32 }
    #[repr(C)] pub struct ImportDescriptor       { pub original_first_thunk: u32, pub time_date_stamp: u32,
                                                   pub forwarder_chain: u32, pub name: u32, pub first_thunk: u32 }
    #[repr(C)] pub struct FileHeader             { pub machine: u16, pub number_of_sections: u16,
                                                   pub time_date_stamp: u32, pub ptr_symbol_table: u32,
                                                   pub number_of_symbols: u32, pub size_of_optional_hdr: u16,
                                                   pub characteristics: u16 }
    #[repr(C)] pub struct SectionHeader          { pub name: [u8;8], pub virtual_size: u32,
                                                   pub virtual_address: u32, pub size_of_raw_data: u32,
                                                   pub pointer_to_raw_data: u32, _pad: [u32;3], pub characteristics: u32 }
    const _: () = assert!(std::mem::size_of::<SectionHeader>() == 40);
    #[repr(C)] pub struct NtHeaders64            { pub signature: u32, pub file_header: FileHeader,
                                                   pub optional_header: OptionalHeader64 }
    #[repr(C)] pub struct OptionalHeader64 {
        pub magic: u16, pub major_linker: u8, pub minor_linker: u8,
        pub size_of_code: u32, pub size_of_init_data: u32, pub size_of_uninit_data: u32,
        pub address_of_entry_point: u32, pub base_of_code: u32,
        pub image_base: u64, pub section_alignment: u32, pub file_alignment: u32,
        pub major_os: u16, pub minor_os: u16, pub major_image: u16, pub minor_image: u16,
        pub major_subsys: u16, pub minor_subsys: u16, pub win32_version: u32,
        pub size_of_image: u32, pub size_of_headers: u32, pub checksum: u32,
        pub subsystem: u16, pub dll_characteristics: u16,
        pub size_of_stack_reserve: u64, pub size_of_stack_commit: u64,
        pub size_of_heap_reserve: u64,  pub size_of_heap_commit: u64,
        pub loader_flags: u32, pub number_of_rva_and_sizes: u32,
        pub data_directory: [DataDirectory; 16],
    }
}

// ---------------------------------------------------------------------------
// CLI entry point
// ---------------------------------------------------------------------------
#[cfg(windows)]
fn run() -> anyhow::Result<()> {
    use std::path::PathBuf;

    let mut log = Log::open();
    log!(log, "=================================================================");
    log!(log, "  ImRuski Manual-Map Injector — debug log");
    log!(log, "  Log file: {LOG_PATH}");
    log!(log, "  Time: {:?}", std::time::SystemTime::now());
    log!(log, "=================================================================");

    let proc_name = "SCP Project Unity.exe".to_string();
    log!(log, "[INPUT] Target process: '{proc_name}'");

    let dll_path = PathBuf::from(r"C:\Users\mouss\Music\imgui2\target\release\imruski_payload.dll");
    log!(log, "[INPUT] DLL path: '{}'", dll_path.display());

    anyhow::ensure!(dll_path.exists(), "DLL not found: {}", dll_path.display());
    let dll_bytes = std::fs::read(&dll_path)?;
    log!(log, "[*] DLL loaded from disk: {} bytes ({} KB)", dll_bytes.len(), dll_bytes.len() / 1024);
    log!(log, "[*] DLL first 4 bytes: {:02X} {:02X} {:02X} {:02X}",
         dll_bytes[0], dll_bytes[1], dll_bytes[2], dll_bytes[3]);

    log!(log, "[*] Searching for '{proc_name}'...");
    let pid = find_process(&proc_name)?;
    log!(log, "[+] Found process '{}' with PID {}", proc_name, pid);

    log!(log, "[*] Starting manual-map injection into PID {}...", pid);
    match manual_map(pid, &dll_bytes, &mut log) {
        Ok(base) => {
            log!(log, "[+] ===== INJECTION SUCCESS =====");
            log!(log, "[+] DLL mapped at remote base: {base:#x}");
            log!(log, "[+] Enjoy ImRuski!");
        }
        Err(ref e) => {
            log!(log, "[!] ===== INJECTION FAILED =====");
            log!(log, "[!] Error: {e:#}");
            return Err(anyhow::anyhow!("{e:#}"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Process discovery
// ---------------------------------------------------------------------------
#[cfg(windows)]
fn find_process(name: &str) -> anyhow::Result<u32> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW,
            PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        },
    };
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)? };
    let mut e = PROCESSENTRY32W { dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
    let target = name.to_lowercase();
    unsafe {
        Process32FirstW(snap, &mut e)?;
        loop {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(260);
            if String::from_utf16_lossy(&e.szExeFile[..len]).to_lowercase() == target {
                let _ = CloseHandle(snap);
                return Ok(e.th32ProcessID);
            }
            if Process32NextW(snap, &mut e).is_err() { break; }
        }
        let _ = CloseHandle(snap);
    }
    anyhow::bail!("Process '{name}' not found")
}

// ---------------------------------------------------------------------------
// Manual mapper
// ---------------------------------------------------------------------------
#[cfg(windows)]
fn manual_map(pid: u32, dll: &[u8], log: &mut Log) -> anyhow::Result<usize> {
    use pe::*;
    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA},
            Memory::{VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE},
            Threading::{CreateRemoteThread, OpenProcess, WaitForSingleObject, PROCESS_ALL_ACCESS},
        },
    };
    use windows::core::PCSTR;

    // -- Parse PE -----------------------------------------------------------------
    anyhow::ensure!(dll.len() > 64, "DLL too small");
    let dos = unsafe { &*(dll.as_ptr() as *const DosHeader) };
    anyhow::ensure!(dos.e_magic == MZ_MAGIC, "Bad MZ magic");
    let nt_off = dos.e_lfanew as usize;
    anyhow::ensure!(nt_off + std::mem::size_of::<NtHeaders64>() <= dll.len());
    let nt = unsafe { &*((dll.as_ptr() as usize + nt_off) as *const NtHeaders64) };
    anyhow::ensure!(nt.signature == PE_SIG, "Bad PE signature");
    anyhow::ensure!(nt.file_header.machine == MACHINE_AMD64, "Only x64 DLLs supported");

    let opt        = &nt.optional_header;
    let img_size   = opt.size_of_image  as usize;
    let pref_base  = opt.image_base     as usize;
    let hdr_size   = opt.size_of_headers as usize;
    let sec_count  = nt.file_header.number_of_sections as usize;
    let sec_off    = nt_off + 4 + std::mem::size_of::<FileHeader>()
                     + nt.file_header.size_of_optional_hdr as usize;

    // -- Open process & allocate --------------------------------------------------
    let hproc = unsafe { OpenProcess(PROCESS_ALL_ACCESS, false, pid)? };
    let _g = defer(move || unsafe { let _ = CloseHandle(hproc); });

    let remote = unsafe {
        let p = VirtualAllocEx(hproc, Some(pref_base as *const _), img_size,
                               MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        if p.is_null() { VirtualAllocEx(hproc, None, img_size, MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE) }
        else { p }
    } as usize;
    anyhow::ensure!(remote != 0, "VirtualAllocEx failed");
    log!(log, "[*] Remote base {remote:#x}  (preferred={pref_base:#x}  delta={:+#x})",
         (remote as i64).wrapping_sub(pref_base as i64));

    // -- Build local image --------------------------------------------------------
    let mut img = vec![0u8; img_size];
    img[..hdr_size.min(dll.len())].copy_from_slice(&dll[..hdr_size.min(dll.len())]);
    for i in 0..sec_count {
        let sec = unsafe { &*((dll.as_ptr() as usize + sec_off + i * std::mem::size_of::<SectionHeader>()) as *const SectionHeader) };
        let raw = sec.pointer_to_raw_data as usize;
        let rsz = sec.size_of_raw_data   as usize;
        let virt= sec.virtual_address    as usize;
        let sname_len = sec.name.iter().position(|&b| b==0).unwrap_or(8);
        let sname = std::str::from_utf8(&sec.name[..sname_len]).unwrap_or("?");
        if rsz == 0 || raw + rsz > dll.len() {
            log!(log, "  [sec {i}] {sname:<10} va={:#010x} vsz={:#x} rsz={:#x} raw={:#010x}  -- SKIPPED",
                 virt, sec.virtual_size, rsz, raw);
            continue;
        }
        let end = (virt + rsz).min(img_size);
        img[virt..end].copy_from_slice(&dll[raw..raw + (end - virt)]);
        log!(log, "  [sec {i}] {sname:<10} va={:#010x} vsz={:#x} rsz={:#x} raw={:#010x}  char={:#010x}  COPIED",
             virt, sec.virtual_size, rsz, raw, sec.characteristics);
    }

    // -- Relocations --------------------------------------------------------------
    let delta = (remote as i64).wrapping_sub(pref_base as i64);
    log!(log, "[*] RELOC delta={delta:+#x}");
    if delta != 0 { fix_relocs(&mut img, opt, delta, log); } else { log!(log, "[*] delta=0, skipping relocs"); }

    // -- Imports ------------------------------------------------------------------
    log!(log, "[*] Resolving imports...");
    resolve_imports(&mut img, opt, log)?;
    log!(log, "[+] Imports resolved");

    // -- Collect TLS callbacks (must be called before DllMain, in place of the OS) -
    let tls_callbacks = collect_tls_callbacks(&img, opt, remote, log);
    log!(log, "[*] TLS callbacks found: {}", tls_callbacks.len());
    for (ti, va) in tls_callbacks.iter().enumerate() { log!(log, "  [tls {ti}] {va:#x}"); }

    // -- Write & execute ----------------------------------------------------------
    unsafe {
        windows::Win32::System::Diagnostics::Debug::WriteProcessMemory(
            hproc, remote as *mut _, img.as_ptr() as *const _, img.len(), None)?;
        log!(log, "[+] WriteProcessMemory OK ({} bytes → {remote:#x})", img.len());
        // Flush the instruction cache so the CPU doesn't execute stale cached bytes
        // from before we wrote the DLL into this memory region.
        {
            type FlushFn = unsafe extern "system" fn(
                *mut core::ffi::c_void, *const core::ffi::c_void, usize) -> i32;
            let k32 = GetModuleHandleA(PCSTR(b"kernel32.dll\0".as_ptr()))?;
            if let Some(fp) = GetProcAddress(k32, PCSTR(b"FlushInstructionCache\0".as_ptr())) {
                let flush: FlushFn = std::mem::transmute(fp);
                let r = flush(hproc.0 as *mut _, remote as *const _, img.len());
                log!(log, "[+] FlushInstructionCache ret={r}");
            } else { log!(log, "[!] FlushInstructionCache not found"); }
        }
    }

    let entry = remote + opt.address_of_entry_point as usize;
    anyhow::ensure!(opt.address_of_entry_point > 0, "No entry point");

    // Primary: read the function pointer from the `.imrski` sentinel section.
    log!(log, "[*] Searching .imrski sentinel section...");
    let init_va = find_init_via_section(dll, &img, opt, remote, sec_off, sec_count, log);
    log!(log, "[*] Scanning export table for 'imruski_init'...");
    let export_init_va = find_export_va(&img, opt, remote, b"imruski_init", log);
    log!(log, "[*] Scanning export table for 'DllMain'...");
    let dll_va         = find_export_va(&img, opt, remote, b"DllMain", log);

    let use_init = init_va.is_some() || export_init_va.is_some();
    let call_target = init_va.or(export_init_va).or(dll_va).unwrap_or(entry);

    if init_va.is_some() {
        log!(log, "[+] INIT: .imrski section → imruski_init at {call_target:#x}");
    } else if export_init_va.is_some() {
        log!(log, "[+] INIT: export table → imruski_init at {call_target:#x}");
    } else if dll_va.is_some() {
        log!(log, "[+] INIT: export table → DllMain at {call_target:#x}");
    } else {
        log!(log, "[!] INIT: FALLBACK to PE entry {call_target:#x}  <-- may crash!");
    }
    log!(log, "[*] use_init={use_init}  call_target={call_target:#x}");

    let exit_thread = unsafe {
        let k32 = GetModuleHandleA(PCSTR(b"kernel32.dll\0".as_ptr()))?;
        let et = GetProcAddress(k32, PCSTR(b"ExitThread\0".as_ptr()))
            .ok_or_else(|| anyhow::anyhow!("ExitThread not found"))? as usize;
        log!(log, "[*] ExitThread VA = {et:#x}");
        et
    };

    // x64 shellcode:
    //   RSP on entry: %16 == 8  (CreateRemoteThread calls us via `call`,
    //   pushing a return address, so RSP is in normal post-call state).
    //   Before every `call` we need RSP%16 == 0.
    //   Strategy: `sub rsp, 0x28` (40 bytes) from RSP%16==8
    //             gives (8-40) % 16 = -32 % 16 = 0  ✓
    //   Do NOT add an extra `sub rsp,8` – that inverts the parity.
    //
    //   Sequence:
    //   1. Each TLS callback: cb(base, 1, NULL)
    //   2a. imruski_init(base)          if init export found
    //   2b. DllMain(base, 1, NULL)      if DllMain export found
    //   2c. entry(base, 1, NULL)        fallback (CRT startup)
    //   3. ExitThread(0)
    let mut sc = Vec::<u8>::with_capacity(70 + tls_callbacks.len() * 28);

    // Emit one call per TLS callback: cb(base, DLL_PROCESS_ATTACH=1, NULL)
    for &cb_va in &tls_callbacks {
        sc.extend_from_slice(&[0x48, 0xB9]); sc.extend_from_slice(&(remote      as u64).to_le_bytes()); // mov rcx, base
        sc.extend_from_slice(&[0xBA, 0x01, 0x00, 0x00, 0x00]);                                          // mov edx, 1
        sc.extend_from_slice(&[0x45, 0x33, 0xC0]);                                                       // xor r8d, r8d
        sc.extend_from_slice(&[0x48, 0xB8]); sc.extend_from_slice(&(cb_va       as u64).to_le_bytes()); // mov rax, cb
        sc.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28, 0xFF, 0xD0, 0x48, 0x83, 0xC4, 0x28]);           // sub/call/add
    }

    if use_init {
        // imruski_init(base): patches TLS slot then calls CRT startup internally
        sc.extend_from_slice(&[0x48, 0xB9]); sc.extend_from_slice(&(remote      as u64).to_le_bytes()); // mov rcx, base
        sc.extend_from_slice(&[0x48, 0xB8]); sc.extend_from_slice(&(call_target as u64).to_le_bytes()); // mov rax, imruski_init
        sc.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28, 0xFF, 0xD0, 0x48, 0x83, 0xC4, 0x28]);           // sub/call/add
    } else {
        // fallback: call PE entry directly (base, DLL_PROCESS_ATTACH=1, NULL)
        sc.extend_from_slice(&[0x48, 0xB9]); sc.extend_from_slice(&(remote as u64).to_le_bytes()); // mov rcx, base
        sc.extend_from_slice(&[0xBA, 0x01, 0x00, 0x00, 0x00]);                                     // mov edx, 1
        sc.extend_from_slice(&[0x45, 0x33, 0xC0]);                                                  // xor r8d, r8d
        sc.extend_from_slice(&[0x48, 0xB8]); sc.extend_from_slice(&(entry as u64).to_le_bytes()); // mov rax, entry
        sc.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28, 0xFF, 0xD0, 0x48, 0x83, 0xC4, 0x28]);     // sub/call/add
    }

    // ExitThread(0) -- after add rsp,0x28 above, RSP%16==8 again;
    // need another sub rsp,0x28 for alignment before this call.
    sc.extend_from_slice(&[0x33, 0xC9]);                                                              // xor ecx, ecx
    sc.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);                                                 // sub rsp, 0x28
    sc.extend_from_slice(&[0x48, 0xB8]); sc.extend_from_slice(&(exit_thread as u64).to_le_bytes()); // mov rax, ExitThread
    sc.extend_from_slice(&[0xFF, 0xD0]);                                                              // call rax

    log!(log, "[*] Shellcode size: {} bytes", sc.len());
    let sc_hex: String = sc.iter().map(|b| format!("{b:02X} ")).collect();
    log!(log, "[*] Shellcode hex: {sc_hex}");
    unsafe {
        let sc_mem = VirtualAllocEx(hproc, None, sc.len(), MEM_COMMIT | MEM_RESERVE, PAGE_EXECUTE_READWRITE);
        anyhow::ensure!(!sc_mem.is_null(), "shellcode alloc failed");
        log!(log, "[*] Shellcode allocated at remote {sc_mem:?}");
        windows::Win32::System::Diagnostics::Debug::WriteProcessMemory(
            hproc, sc_mem, sc.as_ptr() as *const _, sc.len(), None)?;
        log!(log, "[+] Shellcode written");
        log!(log, "[*] CreateRemoteThread → shellcode {sc_mem:?}");
        let t = CreateRemoteThread(hproc, None, 0, Some(std::mem::transmute(sc_mem)), None, 0, None)?;
        log!(log, "[+] Remote thread handle: {t:?}");
        let wait = WaitForSingleObject(t, 8000);
        log!(log, "[+] WaitForSingleObject returned: {wait:?}");
        let _ = CloseHandle(t);
        let _ = VirtualFreeEx(hproc, sc_mem, 0, MEM_RELEASE);
    }
    log!(log, "[+] manual_map done — remote base = {remote:#x}");
    Ok(remote)
}

// ---------------------------------------------------------------------------
// Export table: find a named export VA
// ---------------------------------------------------------------------------
// Rust cdylib (MSVC) normally has an empty PE export table; named exports
// only appear when explicitly requested via /EXPORT linker flag (build.rs).
// `target` must be the exact export name bytes WITHOUT a null terminator.
#[cfg(windows)]
fn find_export_va(img: &[u8], opt: &pe::OptionalHeader64, remote: usize, target: &[u8], log: &mut Log) -> Option<usize> {
    let export_rva = opt.data_directory[0].virtual_address as usize;
    if export_rva == 0 { log!(log, "  [export] no export directory"); return None; }
    // IMAGE_EXPORT_DIRECTORY offsets:
    // +20: NumberOfFunctions  +24: NumberOfNames
    // +28: AddressOfFunctions +32: AddressOfNames  +36: AddressOfNameOrdinals
    if export_rva + 40 > img.len() { return None; }
    let ed = img.as_ptr() as usize + export_rva;
    let num_names    = unsafe { *((ed + 24) as *const u32) } as usize;
    let funcs_rva    = unsafe { *((ed + 28) as *const u32) } as usize;
    let names_rva    = unsafe { *((ed + 32) as *const u32) } as usize;
    let ordinals_rva = unsafe { *((ed + 36) as *const u32) } as usize;
    log!(log, "  [export] export_dir_rva={export_rva:#x} num_names={num_names} funcs_rva={funcs_rva:#x} names_rva={names_rva:#x}");
    for i in 0..num_names {
        let ne_off = names_rva + i * 4;
        if ne_off + 4 > img.len() { break; }
        let name_rva = unsafe { *((img.as_ptr() as usize + ne_off) as *const u32) } as usize;
        if name_rva + target.len() >= img.len() { continue; }
        if &img[name_rva..name_rva + target.len()] != target { continue; }
        if img[name_rva + target.len()] != 0 { continue; }
        let ord_off  = ordinals_rva + i * 2;
        if ord_off + 2 > img.len() { break; }
        let ordinal  = unsafe { *((img.as_ptr() as usize + ord_off) as *const u16) } as usize;
        let func_off = funcs_rva + ordinal * 4;
        if func_off + 4 > img.len() { break; }
        let func_rva = unsafe { *((img.as_ptr() as usize + func_off) as *const u32) } as usize;
        let va = remote + func_rva;
        log!(log, "  [export] FOUND '{}' → rva={func_rva:#x} va={va:#x}",
             String::from_utf8_lossy(target));
        return Some(va);
    }
    log!(log, "  [export] '{}' NOT found (searched {num_names} names)", String::from_utf8_lossy(target));
    None
}

// ---------------------------------------------------------------------------
// Section-based init: read `IMRUSKI_INIT_PTR` from the `.imrski` section
// ---------------------------------------------------------------------------
// The payload places a function pointer (`imruski_init`) into a dedicated
// `.imrski` section.  After fix_relocs the 8 bytes there are the absolute VA
// of `imruski_init` in the remote process.  We read those bytes from our local
// `img` copy (which has relocations already applied) and return the VA.
// `dll`     = original DLL file bytes (section headers are in `dll`, not `img`)
// `img`     = local image copy (headers + sections, relocations applied)
// `sec_off` = byte offset of the first SectionHeader in `dll`
// `sec_count` = number of sections
#[cfg(windows)]
fn find_init_via_section(dll: &[u8], img: &[u8], _opt: &pe::OptionalHeader64,
                         remote: usize, sec_off: usize, sec_count: usize, log: &mut Log) -> Option<usize> {
    for i in 0..sec_count {
        let off = sec_off + i * std::mem::size_of::<pe::SectionHeader>();
        if off + std::mem::size_of::<pe::SectionHeader>() > dll.len() { break; }
        let sec = unsafe { &*(( dll.as_ptr() as usize + off) as *const pe::SectionHeader) };
        // Section names are 8 bytes, NUL-padded.
        let name = &sec.name;
        let nlen = name.iter().position(|&b| b==0).unwrap_or(8);
        let nstr = std::str::from_utf8(&name[..nlen]).unwrap_or("?");
        log!(log, "  [section {i}] '{nstr}' va={:#x}", sec.virtual_address);
        if name.starts_with(b".imrski") {
            let rva = sec.virtual_address as usize;
            log!(log, "  [.imrski] found! rva={rva:#x}");
            if rva + 8 > img.len() {
                log!(log, "  [.imrski] ERROR: rva+8={:#x} > img.len={:#x}", rva+8, img.len());
                return None;
            }
            let fn_va = u64::from_le_bytes(img[rva..rva + 8].try_into().ok()?) as usize;
            log!(log, "  [.imrski] fn_ptr bytes = {:02X?}", &img[rva..rva+8]);
            log!(log, "  [.imrski] fn_va = {fn_va:#x}  remote_range=[{remote:#x}..{:#x}]", remote+img.len());
            if fn_va >= remote && fn_va < remote + img.len() {
                log!(log, "  [.imrski] OK — imruski_init at {fn_va:#x}");
                return Some(fn_va);
            } else {
                log!(log, "  [.imrski] fn_va out of range!");
            }
        }
    }
    log!(log, "  [.imrski] section not found in {} sections", sec_count);
    None
}

// ---------------------------------------------------------------------------
// TLS callback collection
// ---------------------------------------------------------------------------
// The OS normally invokes TLS callbacks (stored in DataDirectory[9]) for every
// DLL before calling DllMain.  For a manually-mapped DLL that step is skipped,
// so the MSVC CRT's _CRT_INIT and any Rust TLS setup routines never run.
// We enumerate the callbacks here and emit an explicit call for each one in the
// remote shellcode, exactly as the OS loader would.
#[cfg(windows)]
fn collect_tls_callbacks(img: &[u8], opt: &pe::OptionalHeader64, remote: usize, log: &mut Log) -> Vec<usize> {
    // IMPORTANT: For a manually-mapped DLL the OS loader never allocated a TLS
    // slot (it never set *AddressOfIndex).  Calling the TLS callback would write
    // our data into TLS slot 0, which belongs to the game/ntdll → instant
    // corruption.  Skip TLS callbacks entirely; Rust std handles its own TLS
    // through lazy_static / thread-local keys at runtime without needing them.
    let dir = &opt.data_directory[pe::DIR_TLS];
    log!(log, "  [tls] dir rva={:#x} size={:#x} -- SKIPPING (no TLS slot allocated for manual-map)",
         dir.virtual_address, dir.size);
    return Vec::new();
    #[allow(unreachable_code)]
    if dir.virtual_address == 0 || dir.size < 40 { log!(log, "  [tls] no TLS directory"); return Vec::new(); }
    let tls_rva = dir.virtual_address as usize;
    if tls_rva + 40 > img.len() { return Vec::new(); }

    // IMAGE_TLS_DIRECTORY64, offset 24 = AddressOfCallBacks (absolute VA)
    let addr_of_cbs = u64::from_le_bytes(
        img[tls_rva + 24..tls_rva + 32].try_into().unwrap()
    ) as usize;
    if addr_of_cbs == 0 { return Vec::new(); }

    // After fix_relocs the VA is already adjusted for `remote`.
    // Derive the RVA to index into our local `img` copy.
    let cbs_rva = addr_of_cbs.wrapping_sub(remote);
    let mut callbacks = Vec::new();
    let mut off = cbs_rva;
    loop {
        if off + 8 > img.len() { break; }
        let cb_va = u64::from_le_bytes(img[off..off + 8].try_into().unwrap()) as usize;
        if cb_va == 0 { break; }
        callbacks.push(cb_va);   // already a remote-process VA
        off += 8;
    }
    callbacks
}

// ---------------------------------------------------------------------------
// Relocation fixup
// ---------------------------------------------------------------------------
#[cfg(windows)]
fn fix_relocs(img: &mut [u8], opt: &pe::OptionalHeader64, delta: i64, log: &mut Log) {
    let dir = &opt.data_directory[pe::DIR_BASERELOC];
    log!(log, "  [reloc] dir rva={:#x} size={:#x} delta={delta:+#x}", dir.virtual_address, dir.size);
    if dir.size == 0 { log!(log, "  [reloc] no reloc dir, skipping"); return; }
    let mut off = dir.virtual_address as usize;
    let end     = off + dir.size as usize;
    while off + 8 <= end.min(img.len()) {
        let base  = u32::from_le_bytes(img[off..off+4].try_into().unwrap()) as usize;
        let bsize = u32::from_le_bytes(img[off+4..off+8].try_into().unwrap()) as usize;
        if bsize < 8 { break; }
        let n = (bsize - 8) / 2;
        for i in 0..n {
            let eo = off + 8 + i * 2;
            if eo + 2 > img.len() { break; }
            let e  = u16::from_le_bytes(img[eo..eo+2].try_into().unwrap());
            if e >> 12 == 10 {
                let rva = base + (e & 0x0FFF) as usize;
                if rva + 8 <= img.len() {
                    let v = i64::from_le_bytes(img[rva..rva+8].try_into().unwrap());
                    img[rva..rva+8].copy_from_slice(&v.wrapping_add(delta).to_le_bytes());
                }
            }
        }
        log!(log, "  [reloc] block va={base:#x} size={bsize:#x} entries={n}");
        off += bsize;
    }
    log!(log, "  [reloc] done");
}

// ---------------------------------------------------------------------------
// Import resolution
// ---------------------------------------------------------------------------
#[cfg(windows)]
fn resolve_imports(img: &mut [u8], opt: &pe::OptionalHeader64, log: &mut Log) -> anyhow::Result<()> {
    use pe::*;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
    use windows::core::PCSTR;

    let dir = &opt.data_directory[DIR_IMPORT];
    if dir.size == 0 { return Ok(()); }

    let dsz = std::mem::size_of::<ImportDescriptor>();
    let mut off = dir.virtual_address as usize;
    loop {
        anyhow::ensure!(off + dsz <= img.len());
        let desc = unsafe { &*(img.as_ptr().add(off) as *const ImportDescriptor) };
        if desc.name == 0 { break; }

        let name_rva = desc.name as usize;
        let dll_name = unsafe { std::ffi::CStr::from_ptr(img.as_ptr().add(name_rva) as *const _) }
            .to_str().unwrap_or("?");
        log!(log, "  [import] DLL: {dll_name}");
        let hmod = unsafe {
            GetModuleHandleA(PCSTR(img.as_ptr().add(name_rva)))
                .or_else(|_| LoadLibraryA(PCSTR(img.as_ptr().add(name_rva))))?
        };

        let src  = if desc.original_first_thunk != 0 { desc.original_first_thunk as usize } else { desc.first_thunk as usize };
        let iat  = desc.first_thunk as usize;
        let mut i = 0usize;
        loop {
            let to = src + i * 8;
            anyhow::ensure!(to + 8 <= img.len());
            let val = u64::from_le_bytes(img[to..to+8].try_into().unwrap());
            if val == 0 { break; }

            let fp = if val & IMAGE_ORDINAL_FLAG64 != 0 {
                unsafe { GetProcAddress(hmod, PCSTR((val & 0xFFFF) as *const u8)) }
            } else {
                let ibn = val as usize + 2; // skip u16 Hint
                anyhow::ensure!(ibn < img.len());
                unsafe { GetProcAddress(hmod, PCSTR(img.as_ptr().add(ibn))) }
            };

            let iate = iat + i * 8;
            if iate + 8 <= img.len() {
                let addr = fp.map(|f| f as u64).unwrap_or(0);
                img[iate..iate+8].copy_from_slice(&addr.to_le_bytes());
                if addr == 0 { log!(log, "[!] Unresolved import at IAT+{iate:#x}"); }
            }
            i += 1;
        }
        off += dsz;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tiny defer helper
// ---------------------------------------------------------------------------
#[cfg(windows)]
struct Defer<F: FnOnce()>(Option<F>);
#[cfg(windows)]
fn defer<F: FnOnce()>(f: F) -> Defer<F> { Defer(Some(f)) }
#[cfg(windows)]
impl<F: FnOnce()> Drop for Defer<F> { fn drop(&mut self) { if let Some(f) = self.0.take() { f(); } } }