//! Slider and drag widgets.

use crate::{
    id::parse_label,
    style::StyleColor,
    ui::Ui,
    Rect, Vec2,
};

pub fn slider_float(ui: &mut Ui<'_>, label: &str, v: &mut f32, min: f32, max: f32) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fs  = ui.ctx.style.font_size;
    let fp  = ui.ctx.style.frame_padding;
    let h   = fs + fp.1 * 2.0;
    let tw  = ui.text_width(text);
    let sp  = ui.ctx.style.item_spacing;

    // Track width = available – label – spacing
    let track_w = (ui.available_width() - tw - sp.0).max(50.0);
    let total   = Vec2::new(track_w + sp.0 + tw, h);

    let pos = match ui.layout_next(total) { Some(p) => p, None => return false };
    let track_rect = Rect::from_min_size(pos, Vec2::new(track_w, h));

    // Interaction
    let (hovered, held, _) = ui.ctx.button_behavior(id, track_rect);
    let mut changed = false;
    if held {
        let t = ((ui.ctx.input.mouse_pos.x - track_rect.min.x) / track_w).clamp(0.0, 1.0);
        let new_v = min + t * (max - min);
        if (*v - new_v).abs() > f32::EPSILON {
            *v = new_v;
            changed = true;
        }
    }

    // Pre-compute text metrics before mutably borrowing draw_list
    let val_str  = format!("{:.3}", v);
    let vw       = ui.text_width(&val_str);
    let tc       = ui.ctx.style.color(StyleColor::Text);
    let vp       = Vec2::new(pos.x + (track_w - vw) * 0.5, pos.y + fp.1);
    let bg       = ui.ctx.style.color(StyleColor::FrameBg);
    let rounding = ui.ctx.style.frame_rounding;
    let t        = if max != min { (*v - min) / (max - min) } else { 0.0 };
    let filled_w = (track_w * t).max(0.0);
    let grab_sz  = ui.ctx.style.grab_min_size.max(6.0);
    let grab_x   = (pos.x + filled_w - grab_sz * 0.5).clamp(pos.x, pos.x + track_w - grab_sz);
    let grab_rect = Rect::from_min_size(Vec2::new(grab_x, pos.y + 1.0), Vec2::new(grab_sz, h - 2.0));
    let fill_col  = if held { ui.ctx.style.color(StyleColor::SliderGrabActive) }
                    else    { ui.ctx.style.color(StyleColor::SliderGrab).with_alpha(0.5) };
    let grab_col  = if held { ui.ctx.style.color(StyleColor::SliderGrabActive) }
                    else    { ui.ctx.style.color(StyleColor::SliderGrab) };

    // Draw track
    {
        let draw = &mut ui.ctx.draw_list;
        draw.filled_rect(track_rect, rounding, bg);
        if filled_w > 0.0 {
            let filled_rect = Rect::from_min_size(pos, Vec2::new(filled_w, h));
            draw.filled_rect(filled_rect, rounding, fill_col);
        }
        draw.filled_rect(grab_rect, rounding, grab_col);
    } // draw borrow ends

    ui.draw_text(&val_str, vp, tc);

    // Label
    let lp = Vec2::new(pos.x + track_w + sp.0, pos.y + (h - fs) * 0.5);
    ui.draw_text(text, lp, tc);

    changed
}

pub fn drag_float(ui: &mut Ui<'_>, label: &str, v: &mut f32, speed: f32, min: f32, max: f32) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fs  = ui.ctx.style.font_size;
    let fp  = ui.ctx.style.frame_padding;
    let h   = fs + fp.1 * 2.0;
    let tw  = ui.text_width(text);
    let sp  = ui.ctx.style.item_spacing;
    let box_w = (ui.available_width() - tw - sp.0).max(50.0);
    let total = Vec2::new(box_w + sp.0 + tw, h);

    let pos = match ui.layout_next(total) { Some(p) => p, None => return false };
    let box_rect = Rect::from_min_size(pos, Vec2::new(box_w, h));

    let (hovered, held, _) = ui.ctx.button_behavior(id, box_rect);
    let mut changed = false;
    if held {
        let delta = ui.ctx.input.mouse_delta.x * speed;
        if delta.abs() > f32::EPSILON {
            *v = (*v + delta).clamp(if min == max { f32::NEG_INFINITY } else { min },
                                    if min == max { f32::INFINITY    } else { max });
            changed = true;
        }
    }

    let bg       = if hovered { ui.ctx.style.color(StyleColor::FrameBgHovered) }
                   else       { ui.ctx.style.color(StyleColor::FrameBg) };
    let rounding = ui.ctx.style.frame_rounding;
    let border   = ui.ctx.style.color(StyleColor::Border);
    let val_str  = format!("{:.3}", v);
    let tc       = ui.ctx.style.color(StyleColor::Text);
    let vp       = Vec2::new(pos.x + fp.0, pos.y + (h - fs) * 0.5);

    {
        let draw = &mut ui.ctx.draw_list;
        draw.filled_rect(box_rect, rounding, bg);
        draw.rect_outline(box_rect, 1.0, border);
    } // draw borrow ends

    ui.draw_text(&val_str, vp, tc);

    let lp = Vec2::new(pos.x + box_w + sp.0, pos.y + (h - fs) * 0.5);
    ui.draw_text(text, lp, tc);

    changed
}
