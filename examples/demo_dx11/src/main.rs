//! demo_dx11 – ImRuski DirectX 11 game-overlay via hudhook.
//!
//! Compiled as a DLL (`crate-type = ["cdylib"]` in a real project),
//! injected into the target process.
//!
//! Here we show it as a binary for documentation/testing purposes.

#[cfg(windows)]
fn main() {
    use imruski::{Context, Vec2, WindowFlags};
    use imruski_dx11::{Dx11Frame, Dx11Hook, ImRuskiRenderLoop};
    use obfstr::obfstr;

    env_logger::init();

    // ── Application state ─────────────────────────────────────────────────────

    struct GameOverlay {
        ctx:        Context,
        open:       bool,
        esp_enable: bool,
        fov:        f32,
        aimbot_key: usize,
        color:      [f32; 4],
        key_names:  Vec<&'static str>,
    }

    impl Default for GameOverlay {
        fn default() -> Self {
            let mut ctx = Context::new();
            ctx.set_display_size(Vec2::new(1920.0, 1080.0));
            // Apply a custom accent colour
            ctx.style_mut().colors[imruski::style::StyleColor::Button as usize] =
                imruski::Color::from_hex(0x9b27af); // purple accent

            Self {
                ctx,
                open:       true,
                esp_enable: false,
                fov:        90.0,
                aimbot_key: 1,
                color:      [1.0, 0.0, 0.3, 1.0],
                key_names:  vec!["None", "Mouse4", "Mouse5", "Shift", "Ctrl", "Alt"],
            }
        }
    }

    impl ImRuskiRenderLoop for GameOverlay {
        fn render(&mut self, frame: &mut Dx11Frame<'_>) {
            self.ctx.new_frame();

            {
                let mut ui = frame.ui(&mut self.ctx);

                if ui.begin(
                    // String is obfuscated at compile time – not visible in binary
                    obfstr!("Game Overlay##main"),
                    Some(&mut self.open),
                    WindowFlags::NO_RESIZE,
                ) {
                    ui.text(obfstr!("ImRuski – DX11 Overlay"));
                    ui.separator();

                    // ESP toggle
                    ui.checkbox(obfstr!("ESP enabled"), &mut self.esp_enable);

                    // FOV slider
                    ui.slider_float(obfstr!("FOV##fov"), &mut self.fov, 60.0, 180.0);

                    // Aimbot key selector
                    let names: Vec<&str> = self.key_names.clone();
                    ui.combo(obfstr!("Aimbot key##key"), &mut self.aimbot_key, &names);

                    // Colour selector
                    ui.color_edit4(obfstr!("ESP colour##col"), &mut self.color);
                    ui.separator();

                    // Progress bar representing FOV / 180
                    ui.progress_bar(self.fov / 180.0, Vec2::new(0.0, 8.0), Some("FOV"));
                    ui.separator();

                    // Tab bar
                    if ui.begin_tab_bar(obfstr!("overlay_tabs")) {
                        if ui.tab_item(obfstr!("Aimbot")) {
                            ui.text(obfstr!("Aimbot settings here."));
                            ui.end_tab_item();
                        }
                        if ui.tab_item(obfstr!("Visuals")) {
                            ui.text(obfstr!("Visuals settings here."));
                            ui.end_tab_item();
                        }
                        if ui.tab_item(obfstr!("Misc")) {
                            ui.text(obfstr!("Misc settings here."));
                            ui.end_tab_item();
                        }
                        ui.end_tab_bar();
                    }
                }
                ui.end();
            }

            frame.submit(self.ctx.end_frame());
        }
    }

    // ── Hook installation ─────────────────────────────────────────────────────
    // In a real DLL you would spawn a thread in DllMain:
    //
    //   #[no_mangle]
    //   unsafe extern "system" fn DllMain(_, reason: u32, _) -> bool {
    //       if reason == 1 {
    //           std::thread::spawn(|| {
    //               Dx11Hook::new(Box::new(GameOverlay::default()))
    //                   .install()
    //                   .expect("hook failed");
    //           });
    //       }
    //       true
    //   }

    Dx11Hook::new(Box::new(GameOverlay::default()))
        .install()
        .expect("DX11 hook installation failed");

    println!("demo_dx11: hook installed (stub mode, no live DX11 device).");
}

#[cfg(not(windows))]
fn main() {
    eprintln!("demo_dx11 is Windows-only.");
}
