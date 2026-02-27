//! Text rendering helpers (standalone functions, used internally by Ui).

use crate::{ui::Ui, Color};

// All public API already lives on `Ui` in ui.rs.
// This module holds any extra formatting helpers.

/// Format and render text with wrapping at `wrap_width` pixels.
pub fn text_wrapped(ui: &mut Ui<'_>, text: &str, wrap_width: f32, col: Color) {
    let fs   = ui.ctx.style.font_size;
    let sp   = ui.ctx.style.item_spacing;
    let mut line    = String::new();
    let mut line_w  = 0.0f32;

    for word in text.split_whitespace() {
        let ww    = ui.text_width(word);
        let space = if line.is_empty() { 0.0 } else { ui.text_width(" ") };
        if !line.is_empty() && line_w + space + ww > wrap_width {
            // flush current line
            if let Some(pos) = ui.layout_next(crate::Vec2::new(line_w, fs)) {
                ui.draw_text(&line, pos, col);
            }
            line.clear();
            line_w = 0.0;
        }
        if !line.is_empty() { line.push(' '); line_w += space; }
        line.push_str(word);
        line_w += ww;
    }
    if !line.is_empty() {
        if let Some(pos) = ui.layout_next(crate::Vec2::new(line_w, fs)) {
            ui.draw_text(&line, pos, col);
        }
    }
}
