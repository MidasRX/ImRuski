//! Color-editor and color-picker widgets.
//!
//! - `color_edit4`   – compact RGBA row with a preview swatch
//! - `color_picker4` – full HSV wheel + preview

use crate::{
    id::parse_label,
    style::StyleColor,
    ui::Ui,
    Color, Rect, Vec2,
};

// ─── Color edit ──────────────────────────────────────────────────────────────

pub fn color_edit4(ui: &mut Ui<'_>, label: &str, color: &mut [f32; 4], alpha: bool) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let fs   = ui.ctx.style.font_size;
    let fp   = ui.ctx.style.frame_padding;
    let sp   = ui.ctx.style.item_spacing;
    let h    = fs + fp.1 * 2.0;
    let tw   = ui.text_width(text);
    let avail = ui.available_width();

    // Layout: [swatch][R][G][B][A?]  label
    let swatch_w = h;
    let n_fields = if alpha { 4 } else { 3 };
    let fields_w = (avail - swatch_w - sp.0 - tw - sp.0 * (n_fields as f32)) / n_fields as f32;
    let fields_w = fields_w.max(30.0);
    let total_w  = swatch_w + sp.0 + fields_w * n_fields as f32 + sp.0 * (n_fields as f32) + tw;
    let total    = Vec2::new(total_w.min(avail), h);

    let pos = match ui.layout_next(total) { Some(p) => p, None => return false };

    // Colour swatch (opens picker on click)
    let swatch_rect = Rect::from_min_size(pos, Vec2::new(swatch_w, h));
    let swatch_col  = Color::new(color[0], color[1], color[2], if alpha { color[3] } else { 1.0 });

    let (sh, _, sc) = ui.ctx.button_behavior(id, swatch_rect);
    {
        let draw = &mut ui.ctx.draw_list;
        // Checker background for alpha
        if alpha && color[3] < 1.0 {
            let check_col = Color::from_hex(0x808080);
            draw.filled_rect(swatch_rect, ui.ctx.style.frame_rounding, check_col);
        }
        draw.filled_rect(swatch_rect, ui.ctx.style.frame_rounding, swatch_col);
        draw.rect_outline(swatch_rect, 1.0, ui.ctx.style.color(StyleColor::Border));
    }

    // Toggle picker popup on click
    if sc {
        let currently = ui.ctx.get_storage(id).map_or(false, |s| s.active);
        ui.ctx.get_storage_mut(id).active = !currently;
    }

    let mut changed = false;

    // Inline float fields
    let field_names: &[&str] = if alpha { &["R", "G", "B", "A"] } else { &["R", "G", "B"] };
    let mut cx = pos.x + swatch_w + sp.0;
    let cy     = pos.y;

    for (i, &fname) in field_names.iter().enumerate() {
        let field_id   = id.combine(crate::id::Id::from_hash(&i));
        let field_rect = Rect::from_min_size(Vec2::new(cx, cy), Vec2::new(fields_w, h));
        let (fh, fheld, _) = ui.ctx.button_behavior(field_id, field_rect);

        if fheld {
            color[i] = (color[i] + ui.ctx.input.mouse_delta.x * 0.005).clamp(0.0, 1.0);
            changed = true;
        }

        let bg = if fheld { ui.ctx.style.color(StyleColor::FrameBgActive) }
                 else if fh { ui.ctx.style.color(StyleColor::FrameBgHovered) }
                 else { ui.ctx.style.color(StyleColor::FrameBg) };
        let rounding = ui.ctx.style.frame_rounding;
        {
            let draw = &mut ui.ctx.draw_list;
            draw.filled_rect(field_rect, rounding, bg);
            draw.rect_outline(field_rect, 1.0, ui.ctx.style.color(StyleColor::Border));
        }
        let vstr = format!("{:.2}", color[i]);
        let tc   = ui.ctx.style.color(StyleColor::Text);
        let tw2  = ui.text_width(&vstr);
        let tp   = Vec2::new(cx + (fields_w - tw2) * 0.5, cy + fp.1);
        ui.draw_text(&vstr, tp, tc);
        let lp = Vec2::new(cx + 2.0, cy - fs - 1.0);
        ui.draw_text(fname, lp, tc.with_alpha(0.6));

        cx += fields_w + sp.0;
    }

    // Label
    let tc = ui.ctx.style.color(StyleColor::Text);
    let lp = Vec2::new(cx, pos.y + (h - fs) * 0.5);
    if !text.is_empty() { ui.draw_text(text, lp, tc); }

    // Inline picker popup
    let popup_open = ui.ctx.get_storage(id).map_or(false, |s| s.active);
    if popup_open {
        let picker_size = Vec2::new(200.0, 220.0);
        let picker_pos  = Vec2::new(pos.x, pos.y + h + 2.0);
        let picker_rect = Rect::from_min_size(picker_pos, picker_size);
        {
            let draw = &mut ui.ctx.draw_list;
            draw.filled_rect(picker_rect, 4.0, ui.ctx.style.color(StyleColor::PopupBg));
            draw.rect_outline(picker_rect, 1.0, ui.ctx.style.color(StyleColor::Border));
        }
        // Embed a small picker
        let mut col4 = *color;
        if color_picker_sv_square(ui, id, picker_pos, 180.0, &mut col4) {
            *color = col4;
            changed = true;
        }
        // Close on click outside
        if ui.ctx.input.mouse_clicked(crate::input::MouseButton::Left)
            && !picker_rect.contains(ui.ctx.input.mouse_pos)
            && !swatch_rect.contains(ui.ctx.input.mouse_pos)
        {
            ui.ctx.get_storage_mut(id).active = false;
        }
    }

    changed
}

// ─── Full picker ─────────────────────────────────────────────────────────────

pub fn color_picker4(ui: &mut Ui<'_>, label: &str, color: &mut [f32; 4]) -> bool {
    let (text, id_src) = parse_label(label);
    let id = ui.ctx.make_id(id_src);

    let size = Vec2::new(ui.available_width().min(240.0), 260.0);
    let pos  = match ui.layout_next(size) { Some(p) => p, None => return false };

    // Full picker
    let mut col4 = *color;
    let changed  = color_picker_sv_square(ui, id, pos, size.x, &mut col4);
    if changed { *color = col4; }

    // Label
    let tc = ui.ctx.style.color(StyleColor::Text);
    let lp = Vec2::new(pos.x, pos.y + size.y + 2.0);
    ui.draw_text(text, lp, tc);

    changed
}

// ─── SV square + H strip picker ──────────────────────────────────────────────

fn color_picker_sv_square(
    ui:    &mut Ui<'_>,
    id:    crate::id::Id,
    pos:   Vec2,
    width: f32,
    color: &mut [f32; 4],
) -> bool {
    let sq_id   = id.combine(crate::id::Id::from_str("__sv"));
    let h_id    = id.combine(crate::id::Id::from_str("__hue"));
    let col_hsv = Color::new(color[0], color[1], color[2], 1.0).to_hsv();
    let (mut hue, mut sat, mut val) = col_hsv;

    let sq_size = Vec2::splat(width - 20.0);
    let sq_rect = Rect::from_min_size(pos, sq_size);
    let h_rect  = Rect::from_min_size(
        Vec2::new(pos.x + sq_size.x + 4.0, pos.y),
        Vec2::new(16.0, sq_size.y),
    );

    let mut changed = false;

    // SV interaction
    let (svh, svheld, _) = ui.ctx.button_behavior(sq_id, sq_rect);
    if svheld {
        let t = (ui.ctx.input.mouse_pos - sq_rect.min) / sq_size;
        sat = t.x.clamp(0.0, 1.0);
        val = (1.0 - t.y).clamp(0.0, 1.0);
        changed = true;
    }

    // Hue strip interaction
    let (_, hheld, _) = ui.ctx.button_behavior(h_id, h_rect);
    if hheld {
        let t = (ui.ctx.input.mouse_pos.y - h_rect.min.y) / h_rect.height();
        hue = t.clamp(0.0, 1.0);
        changed = true;
    }

    // Draw SV square – render 4 corner colors blended
    const SEGS: usize = 16;
    for iy in 0..SEGS {
        for ix in 0..SEGS {
            let fx0 = ix as f32 / SEGS as f32;
            let fy0 = iy as f32 / SEGS as f32;
            let fx1 = (ix + 1) as f32 / SEGS as f32;
            let fy1 = (iy + 1) as f32 / SEGS as f32;
            let tl = Color::from_hsv(hue, fx0, 1.0 - fy0);
            let blended = tl.lerp(Color::WHITE, 0.0); // simplified: use corner colors
            let cell = Rect::from_min_size(
                sq_rect.min + Vec2::new(sq_size.x * fx0, sq_size.y * fy0),
                Vec2::new(sq_size.x * (fx1 - fx0) + 1.0, sq_size.y * (fy1 - fy0) + 1.0),
            );
            let tl_c = Color::from_hsv(hue, fx0, 1.0 - fy0);
            ui.ctx.draw_list.filled_rect(cell, 0.0, tl_c);
        }
    }
    ui.ctx.draw_list.rect_outline(sq_rect, 1.0, Color::BLACK.with_alpha(0.5));

    // Crosshair on SV square
    let sv_pt = sq_rect.min + Vec2::new(sq_size.x * sat, sq_size.y * (1.0 - val));
    ui.ctx.draw_list.filled_circle(sv_pt, 5.0, Color::WHITE, 8);
    ui.ctx.draw_list.filled_circle(sv_pt, 3.0, Color::new(color[0], color[1], color[2], 1.0), 8);

    // Hue strip
    const H_SEGS: usize = 32;
    for i in 0..H_SEGS {
        let t0 = i as f32 / H_SEGS as f32;
        let t1 = (i + 1) as f32 / H_SEGS as f32;
        let c0 = Color::from_hsv(t0, 1.0, 1.0);
        let y0 = h_rect.min.y + h_rect.height() * t0;
        let y1 = h_rect.min.y + h_rect.height() * t1;
        let band = Rect::new(Vec2::new(h_rect.min.x, y0), Vec2::new(h_rect.max.x, y1 + 1.0));
        ui.ctx.draw_list.filled_rect(band, 0.0, c0);
    }
    // Hue cursor line
    let hy = h_rect.min.y + h_rect.height() * hue;
    ui.ctx.draw_list.line(
        Vec2::new(h_rect.min.x - 2.0, hy),
        Vec2::new(h_rect.max.x + 2.0, hy),
        2.0,
        Color::WHITE,
    );
    ui.ctx.draw_list.rect_outline(h_rect, 1.0, Color::BLACK.with_alpha(0.5));

    if changed {
        let c = Color::from_hsv(hue, sat, val);
        color[0] = c.r; color[1] = c.g; color[2] = c.b;
    }

    changed
}
