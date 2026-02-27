//! demo_sciter – ImRuski running on the Sciter HTML-engine backend.
//!
//! Run with:  `cargo run -p demo_sciter`
//!
//! Make sure `sciter.dll` (Windows) is present next to the executable or
//! in a folder pointed to by `SCITER_BIN_FOLDER`.

use imruski::{Context, Renderer, Vec2, WindowFlags};
use imruski_sciter::SciterRenderer;

fn main() {
    env_logger::init();

    // ── Create the Sciter backend ─────────────────────────────────────────────
    let mut renderer = SciterRenderer::new(1280, 720, "ImRuski – Sciter Demo")
        .expect("Failed to create Sciter renderer. Is sciter.dll present?");

    // ── ImRuski context ────────────────────────────────────────────────────────
    let mut ctx  = Context::new();
    ctx.set_display_size(Vec2::new(1280.0, 720.0));

    // ── Application state ─────────────────────────────────────────────────────
    let mut open          = true;
    let mut counter       = 0i32;
    let mut slider_val    = 0.5f32;
    let mut checked       = false;
    let mut text_buf      = String::from("Hello!");
    let mut color         = [0.2f32, 0.7, 0.9, 1.0];
    let mut selected_item = 0usize;
    let items             = ["Option A", "Option B", "Option C", "Option D"];

    // ── Simulated render loop ─────────────────────────────────────────────────
    // In a real Sciter integration this is called from within the Sciter
    // event loop (on_draw handler). Here we demonstrate one synthetic frame.
    let font  = renderer.font_atlas();
    ctx.new_frame();
    {
        let mut ui = imruski::ui::Ui::new(&mut ctx, font, 1.0);

        if ui.begin("Sciter Demo Window", Some(&mut open), WindowFlags::empty()) {
            // ── Text ──────────────────────────────────────────────────────────
            ui.text("Welcome to ImRuski (Sciter backend)!");
            ui.text_disabled("This text is disabled.");
            ui.separator();

            // ── Button + counter ─────────────────────────────────────────────
            if ui.button("Increment") { counter += 1; }
            ui.same_line(8.0);
            ui.text(&format!("counter = {counter}"));
            ui.separator();

            // ── Checkbox ─────────────────────────────────────────────────────
            ui.checkbox("Enable feature", &mut checked);
            ui.separator();

            // ── Sliders ──────────────────────────────────────────────────────
            ui.slider_float("Speed##slider", &mut slider_val, 0.0, 1.0);
            ui.drag_float("Drag##drag", &mut slider_val, 0.01, 0.0, 1.0);
            ui.separator();

            // ── Text input ───────────────────────────────────────────────────
            ui.input_text("Name##input", &mut text_buf);
            ui.separator();

            // ── Combo ────────────────────────────────────────────────────────
            ui.combo("Mode##combo", &mut selected_item, &items);
            ui.separator();

            // ── Colour editor ────────────────────────────────────────────────
            ui.color_edit4("Colour##col", &mut color);
            ui.separator();

            // ── Progress bar ─────────────────────────────────────────────────
            ui.progress_bar(slider_val, Vec2::new(0.0, 0.0), None);
            ui.separator();

            // ── Tab bar ──────────────────────────────────────────────────────
            if ui.begin_tab_bar("tabs##main") {
                if ui.tab_item("Info##tab1") {
                    ui.text("This is the Info tab.");
                    ui.end_tab_item();
                }
                if ui.tab_item("Settings##tab2") {
                    ui.text("This is the Settings tab.");
                    ui.end_tab_item();
                }
                ui.end_tab_bar();
            }
        }
        ui.end();
    }

    // Submit to renderer
    renderer.begin_frame();
    renderer.render(ctx.end_frame());
    renderer.end_frame();

    println!("demo_sciter: one frame rendered successfully.");
    println!("counter = {counter}, slider_val = {slider_val:.3}");
    println!("text = {:?}, checked = {checked}", text_buf);
}
