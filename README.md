# imgui2-rs

A fully native **Rust immediate-mode GUI library** inspired by Dear ImGui, built for game overlays, tools, and creative apps.

---

## Features

| Feature | Crate |
|---------|-------|
| Core widgets (window, button, checkbox, slider, text, input, combo, color-picker, tabs, progress bar …) | `imgui2-core` |
| **Sciter** rendering backend (HTML/CSS engine window) | `imgui2-sciter` |
| **Ultralight** rendering backend (GPU-accelerated web renderer) | `imgui2-ultralight` |
| **DirectX 11 hook** overlay via `hudhook` | `imgui2-dx11` |
| Re-exports + umbrella crate | `imgui2` |

Batteries included:
- [`obfstr`](https://crates.io/crates/obfstr) – compile-time string obfuscation
- [`toy-arms`](https://crates.io/crates/toy-arms) – Windows process memory access (optional)

---

## Quick start

```toml
# Cargo.toml
[dependencies]
imgui2 = { version = "0.1", features = ["dx11"] }
```

```rust
use imgui2::prelude::*;

fn main() {
    let mut ctx = Context::new();
    ctx.set_display_size(Vec2::new(1280.0, 720.0));
    // …attach a backend and call ctx.frame(|ui| { … }) each frame
}
```

---

## Backends

### Sciter
Requires the Sciter SDK DLL (`sciter.dll` / `libsciter.dylib`).  
Set `SCITER_BIN_FOLDER` or place the binary next to the executable.

```toml
imgui2 = { version = "0.1", features = ["sciter"] }
```

### Ultralight
Requires the Ultralight SDK (runtime libraries + `resources/` folder).  
Set `UL_SDK_PATH` to the SDK root before building.

```toml
imgui2 = { version = "0.1", features = ["ultralight"] }
```

### DirectX 11 Overlay (hudhook)
Hooks the game's `IDXGISwapChain::Present` to render the overlay.

```toml
imgui2 = { version = "0.1", features = ["dx11"] }
```

```rust
use imgui2::dx11::Dx11Hook;

#[no_mangle]
unsafe extern "system" fn DllMain(
    _hmodule: *mut std::ffi::c_void,
    reason: u32,
    _reserved: *mut std::ffi::c_void,
) -> bool {
    if reason == 1 {
        std::thread::spawn(|| {
            Dx11Hook::new(Box::new(MyGui::default())).install().unwrap();
        });
    }
    true
}
```

---

## Widget reference

```
ui.begin("Window Title", &mut open, WindowFlags::empty())  → bool
ui.end()
ui.button("Click Me")                                       → bool
ui.checkbox("Enable", &mut enabled)                         → bool
ui.slider_float("Speed", &mut speed, 0.0, 100.0)           → bool
ui.slider_int("Count", &mut count, 1, 32)                  → bool
ui.input_text("Name", &mut name_buf)                        → bool
ui.combo("Mode", &mut selected, &items)                     → bool
ui.color_edit4("Color", &mut color)                        → bool
ui.progress_bar(fraction, size)
ui.separator()
ui.same_line(spacing)
ui.dummy(size)
ui.text("Hello, world!")
ui.text_colored(color, "Colored text")
ui.tooltip(|| { ui.text("Tooltip!"); })
ui.begin_tab_bar("tabs") / ui.tab_item("Tab 1") / ui.end_tab_bar()
```

---

## License

MIT OR Apache-2.0
