//! Tab-bar / tab-item widgets.

use crate::{
    id::parse_label,
    style::StyleColor,
    ui::Ui,
    Rect, Vec2,
};

const TAB_H: f32 = 22.0;

/// Begin a tab bar. Returns `true` if at least one tab is rendered.
pub fn begin_tab_bar(ui: &mut Ui<'_>, id_str: &str) -> bool {
    let id  = ui.ctx.make_id(id_str);
    let bar_id = id.combine(crate::id::Id::from_str("__tabbar"));
    // push the tab bar ID so tab_item can read it
    ui.ctx.id_stack.push(bar_id);

    let avail_w = ui.available_width();
    let sp = ui.ctx.style.item_spacing;
    let pos = match ui.layout_next(Vec2::new(avail_w, TAB_H)) {
        Some(p) => p,
        None => return false,
    };
    // draw background
    let bg = ui.ctx.style.color(StyleColor::Tab);
    ui.ctx.draw_list.filled_rect(
        Rect::from_min_size(pos, Vec2::new(avail_w, TAB_H)),
        0.0,
        bg,
    );
    // Store bar position for tab items
    let s = ui.ctx.get_storage_mut(bar_id);
    s.float[0] = pos.x; // cursor x for next tab
    s.float[1] = pos.y;
    true
}

pub fn end_tab_bar(ui: &mut Ui<'_>) {
    ui.ctx.id_stack.pop(); // pop the tab bar ID
}

/// Render a single tab. Returns `true` if this tab is selected.
pub fn tab_item(ui: &mut Ui<'_>, label: &str) -> bool {
    let (text, id_src) = parse_label(label);
    let item_id = ui.ctx.make_id(id_src);

    // Retrieve bar ID (last pushed) and its stored position
    let bar_id = match ui.ctx.id_stack.last().copied() {
        Some(id) => id,
        None    => return false,
    };

    let bar_x  = ui.ctx.get_storage(bar_id).map_or(0.0, |s| s.float[0]);
    let bar_y  = ui.ctx.get_storage(bar_id).map_or(0.0, |s| s.float[1]);
    let active = ui.ctx.get_storage(bar_id).map_or(false, |s| s.int[0] as u64 == item_id.0);

    let fs    = ui.ctx.style.font_size;
    let fp    = ui.ctx.style.frame_padding;
    let tw    = ui.text_width(text);
    let tab_w = tw + fp.0 * 2.0 + 4.0;

    let tab_rect = Rect::from_min_size(Vec2::new(bar_x, bar_y), Vec2::new(tab_w, TAB_H));

    let (_h, _, clicked) = ui.ctx.button_behavior(item_id, tab_rect);
    if clicked || active {
        if clicked {
            // Record selection using low 64 bits of item_id
            ui.ctx.get_storage_mut(bar_id).int[0] = item_id.0 as i32;
        }
        let tab_col = ui.ctx.style.color(StyleColor::TabActive);
        ui.ctx.draw_list.filled_rect(tab_rect, ui.ctx.style.frame_rounding, tab_col);
    } else {
        let tab_col = if ui.ctx.is_hot(item_id) {
            ui.ctx.style.color(StyleColor::TabHovered)
        } else {
            ui.ctx.style.color(StyleColor::Tab)
        };
        ui.ctx.draw_list.filled_rect(tab_rect, ui.ctx.style.frame_rounding, tab_col);
    }

    // Advance bar cursor
    ui.ctx.get_storage_mut(bar_id).float[0] = bar_x + tab_w + 2.0;

    // Tab label
    let tc = ui.ctx.style.color(StyleColor::Text);
    let tp = Vec2::new(tab_rect.min.x + fp.0, tab_rect.min.y + (TAB_H - fs) * 0.5);
    ui.draw_text(text, tp, tc);

    // Return whether this tab's content should be shown
    // First call with no selection → default to first tab
    let stored = ui.ctx.get_storage(bar_id).map(|s| s.int[0] as u64).unwrap_or(0);
    stored == item_id.0 || stored == 0
}

pub fn end_tab_item(_ui: &mut Ui<'_>) {
    // No state needed
}
