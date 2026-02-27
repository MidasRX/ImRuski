//! demo_ultralight – ImRuski with the Ultralight software-render backend.
//!
//! Run with:  `cargo run -p demo_ultralight`
//!
//! After rendering, `renderer.framebuffer` contains the composited RGBA pixels
//! which can be uploaded to any GPU texture or saved to a PNG.

use imruski::{Context, Renderer, Vec2, WindowFlags};
use imruski_ultralight::UltralightRenderer;

fn main() {
    env_logger::init();

    let mut renderer = UltralightRenderer::new(1280, 720, "ImRuski – Ultralight Demo")
        .expect("Failed to create Ultralight renderer");

    let mut ctx = Context::new();
    ctx.set_display_size(Vec2::new(1280.0, 720.0));

    // Application state
    let mut open       = true;
    let mut slider_val = 0.3f32;
    let mut checked    = true;
    let mut text_buf   = String::from("Ultralight!");
    let mut color      = [0.9f32, 0.4, 0.2, 1.0];

    // Build one frame
    let font = renderer.font_atlas();
    ctx.new_frame();
    {
        let mut ui = imruski::ui::Ui::new(&mut ctx, font, 1.0);

        if ui.begin("Ultralight Demo", Some(&mut open), WindowFlags::empty()) {
            ui.text("ImRuski running with the Ultralight backend!");
            ui.separator();

            ui.checkbox("Feature enabled", &mut checked);
            ui.slider_float("Value##ul_slider", &mut slider_val, 0.0, 1.0);
            ui.progress_bar(slider_val, Vec2::ZERO, None);
            ui.separator();
            ui.input_text("Label##ul_input", &mut text_buf);
            ui.color_edit4("Tint##ul_col", &mut color);
            ui.separator();

            if ui.collapsing("Advanced") {
                ui.text("Collapsed section content here.");
                ui.drag_float("Drag##ul_drag", &mut slider_val, 0.01, 0.0, 1.0);
            }
        }
        ui.end();
    }

    renderer.begin_frame();
    renderer.render(ctx.end_frame());
    renderer.end_frame();

    let non_zero = renderer.framebuffer.iter().any(|&b| b != 0);
    println!(
        "demo_ultralight: frame rendered. Framebuffer has non-zero bytes: {non_zero}. \
         Pixels available at renderer.framebuffer ({} bytes).",
        renderer.framebuffer.len()
    );
}
