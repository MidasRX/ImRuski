//! Button widgets.

use crate::{
    id::parse_label,
    style::StyleColor,
    ui::Ui,
    Color, Rect, Vec2,
};

pub fn button(ui: &mut Ui<'_>, label: &str, mut size: Vec2) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fp   = ui.ctx.style.frame_padding;
    let fs   = ui.ctx.style.font_size;
    let tw   = ui.text_width(text);
    let auto = Vec2::new(tw + fp.0 * 2.0, fs + fp.1 * 2.0);
    if size.x <= 0.0 { size.x = auto.x; }
    if size.y <= 0.0 { size.y = auto.y; }

    let pos = match ui.layout_next(size) { Some(p) => p, None => return false };
    let rect = Rect::from_min_size(pos, size);

    let (hovered, held, clicked) = ui.ctx.button_behavior(id, rect);

    let bg_col  = if held && hovered {
        ui.ctx.style.color(StyleColor::ButtonActive)
    } else if hovered {
        ui.ctx.style.color(StyleColor::ButtonHovered)
    } else {
        ui.ctx.style.color(StyleColor::Button)
    };

    // Compute text metrics BEFORE borrowing draw_list mutably
    let tw        = ui.text_width(text);
    let rounding  = ui.ctx.style.frame_rounding;
    let font_size = ui.ctx.style.font_size;
    let tc        = ui.ctx.style.color(StyleColor::Text);

    {
        let draw = &mut ui.ctx.draw_list;
        draw.filled_rect(rect, rounding, bg_col);
        draw.rect_outline(rect, 1.0, Color::from_hex(0x222222).with_alpha(0.6));
    }

    // Centered text
    let tp = Vec2::new(
        pos.x + (size.x - tw) * 0.5,
        pos.y + (size.y - font_size) * 0.5,
    );
    ui.draw_text(text, tp, tc);

    clicked
}

pub fn small_button(ui: &mut Ui<'_>, label: &str) -> bool {
    let old = ui.ctx.style.frame_padding;
    ui.ctx.style.frame_padding = (0.0, 0.0);
    let r = button(ui, label, Vec2::ZERO);
    ui.ctx.style.frame_padding = old;
    r
}

pub fn collapsing_header(ui: &mut Ui<'_>, label: &str) -> bool {
    let (text, id_src) = parse_label(label);
    let id   = ui.ctx.make_id(id_src);
    let open = ui.ctx.get_storage(id).map_or(true, |s| s.open);

    let width = ui.available_width();
    let h     = ui.ctx.style.font_size + ui.ctx.style.frame_padding.1 * 2.0;
    let pos   = match ui.layout_next(Vec2::new(width, h)) { Some(p) => p, None => return open };
    let rect  = Rect::from_min_size(pos, Vec2::new(width, h));

    let (hovered, _, clicked) = ui.ctx.button_behavior(id, rect);

    // Toggle on click
    if clicked {
        let open_new = !open;
        ui.ctx.get_storage_mut(id).open = open_new;
    }

    let bg = if hovered {
        ui.ctx.style.color(StyleColor::HeaderHovered)
    } else {
        ui.ctx.style.color(StyleColor::Header)
    };

    let rounding = ui.ctx.style.frame_rounding;
    let draw = &mut ui.ctx.draw_list;
    draw.filled_rect(rect, rounding, bg);

    // Arrow
    let arrow_x = pos.x + 6.0;
    let arrow_y = pos.y + h * 0.5;
    let (a, b, c) = if open {
        (Vec2::new(arrow_x, arrow_y - 3.0),
         Vec2::new(arrow_x + 6.0, arrow_y - 3.0),
         Vec2::new(arrow_x + 3.0, arrow_y + 3.0))
    } else {
        (Vec2::new(arrow_x, arrow_y - 4.0),
         Vec2::new(arrow_x + 6.0, arrow_y),
         Vec2::new(arrow_x, arrow_y + 4.0))
    };
    let tc = ui.ctx.style.color(StyleColor::Text);
    draw.triangle_filled(a, b, c, tc);
    drop(draw);

    let tp = Vec2::new(pos.x + 20.0, pos.y + (h - ui.ctx.style.font_size) * 0.5);
    ui.draw_text(text, tp, tc);

    // Return the CURRENT open state (before toggle takes effect next frame)
    open
}
