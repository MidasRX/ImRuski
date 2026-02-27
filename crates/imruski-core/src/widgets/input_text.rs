//! Single-line / multi-line text input widget.

use crate::{
    id::parse_label,
    input::Key,
    style::StyleColor,
    ui::Ui,
    Rect, Vec2,
};

pub fn input_text(ui: &mut Ui<'_>, label: &str, buf: &mut String, multiline: bool) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fs  = ui.ctx.style.font_size;
    let fp  = ui.ctx.style.frame_padding;
    let h   = if multiline { fs * 4.0 + fp.1 * 2.0 } else { fs + fp.1 * 2.0 };
    let tw  = ui.text_width(text);
    let sp  = ui.ctx.style.item_spacing;
    let box_w = (ui.available_width() - tw - sp.0).max(60.0);
    let total = Vec2::new(box_w + sp.0 + tw, h);

    let pos = match ui.layout_next(total) { Some(p) => p, None => return false };
    let box_rect = Rect::from_min_size(pos, Vec2::new(box_w, h));

    let focused = ui.ctx.focus_item == Some(id);
    let (hovered, _, clicked) = ui.ctx.button_behavior(id, box_rect);
    if clicked { ui.ctx.focus_item = Some(id); }

    let mut changed = false;

    if focused {
        // Text input from keyboard
        if !ui.ctx.input.text_input.is_empty() {
            buf.push_str(&ui.ctx.input.text_input.clone());
            changed = true;
        }
        // Backspace
        if ui.ctx.input.key_pressed(Key::Backspace) && !buf.is_empty() {
            buf.pop();
            changed = true;
        }
        // Ctrl+A → clear (simple)
        if ui.ctx.input.ctrl() && ui.ctx.input.key_pressed(Key::A) {
            buf.clear();
            changed = true;
        }
        // Escape → lose focus
        if ui.ctx.input.key_pressed(Key::Escape) {
            ui.ctx.focus_item = None;
        }
        // Enter → lose focus (single-line)
        if !multiline && ui.ctx.input.key_pressed(Key::Enter) {
            ui.ctx.focus_item = None;
        }
    }

    // Draw background
    let bg_col = if focused  { ui.ctx.style.color(StyleColor::FrameBgActive)  }
                 else if hovered { ui.ctx.style.color(StyleColor::FrameBgHovered) }
                 else  { ui.ctx.style.color(StyleColor::FrameBg) };
    let rounding = ui.ctx.style.frame_rounding;
    let border_col = if focused {
        ui.ctx.style.color(StyleColor::SliderGrab)
    } else {
        ui.ctx.style.color(StyleColor::Border)
    };
    {
        let draw = &mut ui.ctx.draw_list;
        draw.filled_rect(box_rect, rounding, bg_col);
        draw.rect_outline(box_rect, if focused { 2.0 } else { 1.0 }, border_col);
    }

    // Cursor
    let tc = ui.ctx.style.color(StyleColor::Text);
    let display_text = if buf.len() > 200 { &buf[buf.len()-200..] } else { buf.as_str() };
    let tp = Vec2::new(pos.x + fp.0, pos.y + (h - fs) * 0.5);
    ui.draw_text(display_text, tp, tc);

    if focused {
        // Blinking cursor line
        let cx = pos.x + fp.0 + ui.text_width(display_text);
        let cy1 = pos.y + fp.1 + 1.0;
        let cy2 = pos.y + h - fp.1 - 1.0;
        let blink = ((ui.ctx.input.frame_count / 30) % 2) == 0;
        if blink {
            let cc = ui.ctx.style.color(StyleColor::Text);
            ui.ctx.draw_list.line(Vec2::new(cx, cy1), Vec2::new(cx, cy2), 1.0, cc);
        }
    }

    // Label
    if !text.is_empty() {
        let lp = Vec2::new(pos.x + box_w + sp.0, pos.y + (h - fs) * 0.5);
        ui.draw_text(text, lp, tc);
    }

    changed
}
