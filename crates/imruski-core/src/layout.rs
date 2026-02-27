//! Layout engine – tracks the cursor and handles same-line, dummy, indent, etc.

use crate::Vec2;

/// Layout flow direction (mostly Vertical; Horizontal for same-line groups).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutDir { #[default] Vertical, Horizontal }

/// Per-window cursor state.
#[derive(Debug, Clone, Default)]
pub struct Layout {
    /// Current item insertion point (top-left of next widget).
    pub cursor: Vec2,
    /// Starting cursor X (for same-line baseline).
    pub start_x: f32,
    /// Last item bounding box (for same-line spacing).
    pub last_item_max: Vec2,
    /// Previous cursor Y (for same-line).
    pub prev_line_height: f32,
    /// Indent depth.
    pub indent: f32,
    /// Content region size available (set from window).
    pub content_size: Vec2,
    /// How much content was actually used (for auto-sizing).
    pub content_max: Vec2,
    /// Current direction.
    pub dir: LayoutDir,
    /// Stack for same-line state.
    same_line: bool,
}

impl Layout {
    pub fn new(start: Vec2, width: f32) -> Self {
        Self {
            cursor:    start,
            start_x:   start.x,
            last_item_max: start,
            content_size:  Vec2::new(width, f32::MAX),
            content_max:   start,
            ..Default::default()
        }
    }

    /// Available width for the next widget.
    pub fn available_width(&self) -> f32 {
        (self.start_x + self.content_size.x - self.cursor.x).max(1.0)
    }

    /// Advance cursor after placing a widget of `size`.
    /// Returns the widget's top-left position.
    pub fn place(&mut self, size: Vec2, item_spacing: (f32, f32)) -> Vec2 {
        let pos = self.cursor;
        self.last_item_max = pos + size;
        self.content_max   = Vec2::new(
            self.content_max.x.max(pos.x + size.x),
            self.content_max.y.max(pos.y + size.y),
        );

        if self.dir == LayoutDir::Horizontal {
            self.cursor.x = pos.x + size.x + item_spacing.0;
            self.prev_line_height = self.prev_line_height.max(size.y);
        } else {
            self.cursor.y = pos.y + size.y + item_spacing.1;
            self.cursor.x = self.start_x + self.indent;
        }
        self.same_line = false;
        pos
    }

    /// `same_line` – continue placing items on the current line.
    pub fn same_line(&mut self, spacing: f32) {
        let spacing = if spacing < 0.0 { 8.0 } else { spacing };
        self.cursor.x = self.last_item_max.x + spacing;
        self.cursor.y = self.last_item_max.y - self.prev_line_height;
        self.dir = LayoutDir::Horizontal;
        self.same_line = true;
    }

    /// Advance cursor to the next line without placing a widget.
    pub fn new_line(&mut self, item_spacing: (f32, f32)) {
        self.cursor.y = self.last_item_max.y + item_spacing.1;
        self.cursor.x = self.start_x + self.indent;
        self.dir = LayoutDir::Vertical;
    }

    /// Insert blank space.
    pub fn dummy(&mut self, size: Vec2, item_spacing: (f32, f32)) {
        self.place(size, item_spacing);
    }

    pub fn indent(&mut self, amount: f32) {
        self.indent  += amount;
        self.cursor.x = self.start_x + self.indent;
    }

    pub fn unindent(&mut self, amount: f32) {
        self.indent   = (self.indent - amount).max(0.0);
        self.cursor.x = self.start_x + self.indent;
    }
}
