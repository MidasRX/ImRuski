//! Combo-box (dropdown) widget.

use crate::{
    id::parse_label,
    style::StyleColor,
    ui::Ui,
    Rect, Vec2,
};

pub fn combo(
    ui:       &mut Ui<'_>,
    label:    &str,
    selected: &mut usize,
    items:    &[&str],
) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fs    = ui.ctx.style.font_size;
    let fp    = ui.ctx.style.frame_padding;
    let sp    = ui.ctx.style.item_spacing;
    let h     = fs + fp.1 * 2.0;
    let tw    = ui.text_width(text);
    let box_w = (ui.available_width() - tw - sp.0).max(60.0);
    let total = Vec2::new(box_w + sp.0 + tw, h);

    let pos = match ui.layout_next(total) { Some(p) => p, None => return false };
    let box_rect = Rect::from_min_size(pos, Vec2::new(box_w, h));

    // Is the popup open?
    let open = ui.ctx.get_storage(id).map_or(false, |s| s.active);
    let (_hov, _, clicked) = ui.ctx.button_behavior(id, box_rect);
    let open = if clicked { !open } else { open };
    ui.ctx.get_storage_mut(id).active = open;

    // Draw combo box
    let bg = ui.ctx.style.color(StyleColor::FrameBg);
    let rounding = ui.ctx.style.frame_rounding;
    {
        let draw = &mut ui.ctx.draw_list;
        draw.filled_rect(box_rect, rounding, bg);
        draw.rect_outline(box_rect, 1.0, ui.ctx.style.color(StyleColor::Border));

        // Arrow
        let ax = pos.x + box_w - fp.0 - 6.0;
        let ay = pos.y + h * 0.5;
        let tc_arrow = ui.ctx.style.color(StyleColor::Text);
        draw.triangle_filled(
            Vec2::new(ax - 4.0, ay - 2.0),
            Vec2::new(ax + 4.0, ay - 2.0),
            Vec2::new(ax, ay + 4.0),
            tc_arrow,
        );
    }

    // Current selection text
    let cur = items.get(*selected).copied().unwrap_or("");
    let tc  = ui.ctx.style.color(StyleColor::Text);
    let tp  = Vec2::new(pos.x + fp.0, pos.y + fp.1);
    ui.draw_text(cur, tp, tc);

    // Label
    if !text.is_empty() {
        let lp = Vec2::new(pos.x + box_w + sp.0, pos.y + (h - fs) * 0.5);
        ui.draw_text(text, lp, tc);
    }

    let mut changed = false;

    if open && !items.is_empty() {
        // Draw popup
        let popup_h = items.len() as f32 * (fs + sp.1) + fp.1 * 2.0;
        let popup_rect = Rect::from_min_size(
            Vec2::new(pos.x, pos.y + h + 2.0),
            Vec2::new(box_w, popup_h),
        );
        {
            let draw = &mut ui.ctx.draw_list;
            let popup_bg = ui.ctx.style.color(StyleColor::PopupBg);
            draw.filled_rect(popup_rect, rounding, popup_bg);
            draw.rect_outline(popup_rect, 1.0, ui.ctx.style.color(StyleColor::Border));
        }

        let item_h = fs + sp.1;
        for (i, &item) in items.iter().enumerate() {
            let item_pos  = Vec2::new(popup_rect.min.x, popup_rect.min.y + fp.1 + i as f32 * item_h);
            let item_rect = Rect::from_min_size(item_pos, Vec2::new(box_w, item_h));
            let item_id   = id.combine(crate::id::Id::from_hash(&i));

            let (hovitem, _, clkitem) = ui.ctx.button_behavior(item_id, item_rect);

            if hovitem || i == *selected {
                let hc = if i == *selected {
                    ui.ctx.style.color(StyleColor::HeaderActive)
                } else {
                    ui.ctx.style.color(StyleColor::HeaderHovered)
                };
                ui.ctx.draw_list.filled_rect(item_rect, 0.0, hc);
            }

            if clkitem {
                *selected = i;
                changed = true;
                ui.ctx.get_storage_mut(id).active = false;
            }

            let tp = Vec2::new(item_pos.x + fp.0, item_pos.y + (item_h - fs) * 0.5);
            let tc = ui.ctx.style.color(StyleColor::Text);
            ui.draw_text(item, tp, tc);
        }

        // Close popup if click outside
        if ui.ctx.input.mouse_clicked(crate::input::MouseButton::Left)
            && !popup_rect.contains(ui.ctx.input.mouse_pos)
            && !box_rect.contains(ui.ctx.input.mouse_pos)
        {
            ui.ctx.get_storage_mut(id).active = false;
        }
    }

    changed
}
