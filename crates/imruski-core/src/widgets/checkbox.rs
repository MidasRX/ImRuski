//! Checkbox widget.

use crate::{
    id::parse_label,
    style::StyleColor,
    ui::Ui,
    Rect, Vec2,
};

pub fn checkbox(ui: &mut Ui<'_>, label: &str, v: &mut bool) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fs = ui.ctx.style.font_size;
    let box_sz = fs;
    let sp     = ui.ctx.style.item_spacing;
    let tw     = ui.text_width(text);
    let total  = Vec2::new(box_sz + sp.0 + tw, box_sz);

    let pos = match ui.layout_next(total) { Some(p) => p, None => return false };
    let box_rect = Rect::from_min_size(pos, Vec2::splat(box_sz));

    let (hovered, _, clicked) = ui.ctx.button_behavior(id, Rect::from_min_size(pos, total));
    if clicked { *v = !*v; }

    let bg = if hovered {
        ui.ctx.style.color(StyleColor::FrameBgHovered)
    } else {
        ui.ctx.style.color(StyleColor::FrameBg)
    };
    let rounding = ui.ctx.style.frame_rounding;
    let draw = &mut ui.ctx.draw_list;
    draw.filled_rect(box_rect, rounding, bg);
    draw.rect_outline(box_rect, 1.0, ui.ctx.style.color(StyleColor::Border));

    if *v {
        // Draw checkmark
        let s  = box_sz;
        let cx = pos.x + s * 0.5;
        let cy = pos.y + s * 0.5;
        let ck = ui.ctx.style.color(StyleColor::CheckMark);
        draw.line(
            Vec2::new(cx - s * 0.3, cy),
            Vec2::new(cx - s * 0.05, cy + s * 0.3),
            2.0,
            ck,
        );
        draw.line(
            Vec2::new(cx - s * 0.05, cy + s * 0.3),
            Vec2::new(cx + s * 0.35, cy - s * 0.25),
            2.0,
            ck,
        );
    }

    let tc  = ui.ctx.style.color(StyleColor::Text);
    let tp  = Vec2::new(pos.x + box_sz + sp.0, pos.y + (box_sz - fs) * 0.5);
    drop(draw);
    ui.draw_text(text, tp, tc);

    clicked
}
