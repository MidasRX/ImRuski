//! Progress-bar widget.

use crate::{
    style::StyleColor,
    ui::Ui,
    Rect, Vec2,
};

pub fn progress_bar(ui: &mut Ui<'_>, fraction: f32, mut size: Vec2, overlay: Option<&str>) {
    let fs  = ui.ctx.style.font_size;
    let fp  = ui.ctx.style.frame_padding;
    let h   = fs + fp.1 * 2.0;

    if size.x <= 0.0 { size.x = ui.available_width(); }
    if size.y <= 0.0 { size.y = h; }

    let pos = match ui.layout_next(size) { Some(p) => p, None => return };
    let rect = Rect::from_min_size(pos, size);

    let rounding = ui.ctx.style.frame_rounding;
    let bg_col   = ui.ctx.style.color(StyleColor::FrameBg);
    let fill_col = ui.ctx.style.color(StyleColor::ProgressBar);

    let draw = &mut ui.ctx.draw_list;
    draw.filled_rect(rect, rounding, bg_col);

    let fill_w = (size.x * fraction.clamp(0.0, 1.0)).max(0.0);
    if fill_w > 0.0 {
        let fill_rect = Rect::from_min_size(pos, Vec2::new(fill_w, size.y));
        draw.filled_rect(fill_rect, rounding, fill_col);
    }

    draw.rect_outline(rect, 1.0, ui.ctx.style.color(StyleColor::Border));
    drop(draw);

    // Overlay text (percentage or custom)
    let ov_str = overlay.map(str::to_owned).unwrap_or_else(|| format!("{:.0}%", fraction * 100.0));
    let ow = ui.text_width(&ov_str);
    let tp = Vec2::new(pos.x + (size.x - ow) * 0.5, pos.y + (size.y - fs) * 0.5);
    let tc = ui.ctx.style.color(StyleColor::Text);
    ui.draw_text(&ov_str, tp, tc);
}
