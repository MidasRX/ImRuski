//! Window widget – begin / end pair.

use crate::{
    context::{WindowFrame, WindowState},
    id::parse_label,
    layout::Layout,
    style::StyleColor,
    ui::Ui,
    Color, Rect, Vec2, WindowFlags,
};

const TITLE_BAR_H:  f32 = 22.0;
const RESIZE_GRIP:  f32 = 10.0;
const CLOSE_BTN_SZ: f32 = 14.0;

pub fn begin(
    ui:    &mut Ui<'_>,
    title: &str,
    mut open:  Option<&mut bool>,
    flags: WindowFlags,
) -> bool {
    let (display_name, id_src) = parse_label(title);
    let win_id = ui.ctx.make_id(id_src);

    let display = ui.ctx.input.display_size;
    let default_pos  = Vec2::new(20.0, 20.0);
    let default_size = Vec2::new(300.0, 200.0);

    // Retrieve or create persistent window state
    let ws = ui.ctx.windows.entry(win_id).or_insert_with(|| {
        WindowState::new(default_pos, default_size, flags)
    }).clone();

    // ── Title bar interaction ─────────────────────────────────────────────────

    let title_rect = Rect::from_min_size(ws.pos, Vec2::new(ws.size.x, TITLE_BAR_H));
    let drag_id    = win_id.combine(crate::id::Id::from_str("__drag"));

    // Dragging
    if !flags.contains(WindowFlags::NO_MOVE) {
        let hovered = title_rect.contains(ui.ctx.input.mouse_pos);
        if hovered && ui.ctx.input.mouse_clicked(crate::input::MouseButton::Left) {
            ui.ctx.active_item = Some(drag_id);
        }
        if ui.ctx.active_item == Some(drag_id) {
            let delta = ui.ctx.input.mouse_delta;
            if let Some(w) = ui.ctx.windows.get_mut(&win_id) {
                w.pos += delta;
                // Clamp to display
                w.pos.x = w.pos.x.clamp(0.0, display.x - 40.0);
                w.pos.y = w.pos.y.clamp(0.0, display.y - 40.0);
            }
        }
    }

    // Resize (bottom-right grip)
    let ws2 = ui.ctx.windows.get(&win_id).cloned().unwrap_or_else(|| ws.clone());
    if !flags.contains(WindowFlags::NO_RESIZE) {
        let grip_rect = Rect::from_min_size(
            ws2.pos + ws2.size - Vec2::splat(RESIZE_GRIP),
            Vec2::splat(RESIZE_GRIP),
        );
        let grip_id = win_id.combine(crate::id::Id::from_str("__resize"));
        if grip_rect.contains(ui.ctx.input.mouse_pos)
            && ui.ctx.input.mouse_clicked(crate::input::MouseButton::Left)
        {
            ui.ctx.active_item = Some(grip_id);
        }
        if ui.ctx.active_item == Some(grip_id) {
            let delta = ui.ctx.input.mouse_delta;
            if let Some(w) = ui.ctx.windows.get_mut(&win_id) {
                w.size += delta;
                w.size.x = w.size.x.max(80.0);
                w.size.y = w.size.y.max(40.0);
            }
        }
    }

    // Close button
    if open.is_some() && !flags.contains(WindowFlags::NO_CLOSE_BUTTON) {
        let close_pos = Vec2::new(ws2.pos.x + ws2.size.x - CLOSE_BTN_SZ - 4.0, ws2.pos.y + (TITLE_BAR_H - CLOSE_BTN_SZ) * 0.5);
        let close_rect = Rect::from_min_size(close_pos, Vec2::splat(CLOSE_BTN_SZ));
        let close_id   = win_id.combine(crate::id::Id::from_str("__close"));
        let (_h, _hold, clicked) = ui.ctx.button_behavior(close_id, close_rect);
        if clicked {
            if let Some(ref mut o) = open { **o = false; }
        }
    }

    // Re-read final state
    let ws = ui.ctx.windows.get(&win_id).cloned().unwrap_or_else(|| ws.clone());
    let visible = open.as_ref().map_or(true, |o| **o);

    // ── Drawing ───────────────────────────────────────────────────────────────

    // Pre-extract style values so we can drop borrows before calling ui methods
    let window_rounding  = ui.ctx.style.window_rounding;
    let window_padding   = ui.ctx.style.window_padding;
    let font_size        = ui.ctx.style.font_size;

    {
        let draw  = &mut ui.ctx.draw_list;
        let style = &ui.ctx.style;

        // Background
        if !flags.contains(WindowFlags::NO_BACKGROUND) {
            draw.filled_rect(ws.rect(), style.window_rounding, style.color(StyleColor::WindowBg));
            draw.rect_outline(ws.rect(), 1.0, style.color(StyleColor::WindowBorder));
        }

        // Title bar
        if !flags.contains(WindowFlags::NO_TITLE_BAR) {
            let tb_rect = Rect::from_min_size(ws.pos, Vec2::new(ws.size.x, TITLE_BAR_H));
            draw.filled_rect(tb_rect, style.window_rounding, style.color(StyleColor::TitleBar));
        }

        // Resize grip
        if !flags.contains(WindowFlags::NO_RESIZE) {
            let rg = ws.pos + ws.size - Vec2::splat(RESIZE_GRIP);
            draw.triangle_filled(
                rg + Vec2::new(RESIZE_GRIP, 0.0),
                rg + Vec2::new(0.0, RESIZE_GRIP),
                rg + Vec2::splat(RESIZE_GRIP),
                style.color(StyleColor::ResizeGrip),
            );
        }
    } // draw and style borrows end here

    // Push window frame onto the stack
    let content_start = Vec2::new(
        ws.pos.x + window_padding.0,
        ws.pos.y + (if flags.contains(WindowFlags::NO_TITLE_BAR) { 0.0 } else { TITLE_BAR_H }) + window_padding.1,
    );
    let content_width = ws.size.x - window_padding.0 * 2.0;
    let layout = Layout::new(content_start, content_width);

    let draw_start = ui.ctx.draw_list.cmd_buf.len();
    ui.ctx.window_stack.push(WindowFrame { id: win_id, layout, draw_start });

    // Draw title text
    if !flags.contains(WindowFlags::NO_TITLE_BAR) && visible && !display_name.is_empty() {
        let tp = Vec2::new(
            ws.pos.x + window_padding.0,
            ws.pos.y + (TITLE_BAR_H - font_size) * 0.5,
        );
        let text_col = ui.ctx.style.color(StyleColor::TitleBarText);
        ui.draw_text(display_name, tp, text_col);
    }

    visible && !ws.collapsed
}

pub fn end(ui: &mut Ui<'_>) {
    if let Some(_frame) = ui.ctx.window_stack.pop() {
        // Clip rect was pushed in begin; pop it now
    }
}
