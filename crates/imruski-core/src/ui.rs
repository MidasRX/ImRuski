//! Main immediate-mode UI API surface.
//!
//! All widget calls go through `Ui`, which borrows `Context` mutably.

use crate::{
    context::{Context, WindowFrame, WindowState},
    draw_list::TextureId,
    id::{parse_label, Id},
    layout::Layout,
    renderer::FontAtlas,
    style::StyleColor,
    Color, Rect, Vec2, WindowFlags,
};

// ─── Ui ──────────────────────────────────────────────────────────────────────

/// The immediate-mode API handle. Obtain one via `Context::frame()`.
pub struct Ui<'ctx> {
    pub(crate) ctx:   &'ctx mut Context,
    pub(crate) font:  &'ctx dyn FontAtlas,
    pub(crate) scale: f32,
}

impl<'ctx> Ui<'ctx> {
    #[doc(hidden)]
    pub fn new(ctx: &'ctx mut Context, font: &'ctx dyn FontAtlas, scale: f32) -> Self {
        Self { ctx, font, scale }
    }

    // ── Style / Theme ─────────────────────────────────────────────────────────

    pub fn style(&self) -> &crate::style::Style { &self.ctx.style }

    pub fn push_style_color(&mut self, var: StyleColor, col: Color) -> StyleColorToken {
        let old = self.ctx.style.colors[var as usize];
        self.ctx.style.colors[var as usize] = col;
        StyleColorToken { var, old }
    }

    pub fn pop_style_color(&mut self, tok: StyleColorToken) {
        self.ctx.style.colors[tok.var as usize] = tok.old;
    }

    pub fn push_style_var(&mut self, var: crate::style::StyleVar) -> StyleVarToken {
        let r = self.ctx.style.push_var(var);
        StyleVarToken(r)
    }

    pub fn pop_style_var(&mut self, tok: StyleVarToken) {
        self.ctx.style.pop_var(tok.0);
    }

    // ── Layout ────────────────────────────────────────────────────────────────

    /// Skip to next widget on the same horizontal line.
    pub fn same_line(&mut self, spacing: f32) {
        if let Some(l) = self.ctx.current_layout_mut() { l.same_line(spacing); }
    }

    /// Insert blank space.
    pub fn dummy(&mut self, size: Vec2) {
        let sp = self.ctx.style.item_spacing;
        if let Some(l) = self.ctx.current_layout_mut() { l.dummy(size, sp); }
    }

    /// Explicit line break.
    pub fn new_line(&mut self) {
        let sp = self.ctx.style.item_spacing;
        if let Some(l) = self.ctx.current_layout_mut() { l.new_line(sp); }
    }

    pub fn indent(&mut self)   { let v = self.ctx.style.indent_spacing; if let Some(l) = self.ctx.current_layout_mut() { l.indent(v); } }
    pub fn unindent(&mut self) { let v = self.ctx.style.indent_spacing; if let Some(l) = self.ctx.current_layout_mut() { l.unindent(v); } }

    /// Remaining width available on the current line.
    pub fn available_width(&self) -> f32 {
        self.ctx.current_window()
            .map(|w| w.layout.available_width())
            .unwrap_or(self.ctx.input.display_size.x)
    }

    // ── Display size / delta time ─────────────────────────────────────────────

    pub fn display_size(&self) -> Vec2  { self.ctx.input.display_size }
    pub fn delta_time(&self)   -> f32   { self.ctx.delta_time }
    pub fn frame_count(&self)  -> u64   { self.ctx.input.frame_count }

    // ── ID helpers ────────────────────────────────────────────────────────────

    pub fn push_id_str(&mut self, s: &str) { self.ctx.push_id_str(s); }
    pub fn pop_id(&mut self)               { self.ctx.pop_id(); }

    // ── Text ──────────────────────────────────────────────────────────────────

    pub fn text(&mut self, s: &str) {
        self.text_colored(self.ctx.style.color(StyleColor::Text), s);
    }

    pub fn text_colored(&mut self, color: Color, text: &str) {
        if let Some(pos) = self.layout_next_for_text(text) {
            self.draw_text(text, pos, color);
        }
    }

    pub fn text_disabled(&mut self, s: &str) {
        let c = self.ctx.style.color(StyleColor::TextDisabled);
        self.text_colored(c, s);
    }

    // ── Separator ─────────────────────────────────────────────────────────────

    pub fn separator(&mut self) {
        let width   = self.available_width();
        let sp      = self.ctx.style.item_spacing;
        let col     = self.ctx.style.color(StyleColor::Separator);
        if let Some(pos) = self.ctx.current_layout_mut().map(|l| l.place(Vec2::new(width, 1.0), sp)) {
            let draw = &mut self.ctx.draw_list;
            draw.filled_rect(
                Rect::from_min_size(pos, Vec2::new(width, 1.0)),
                0.0,
                col,
            );
        }
    }

    // ── Tooltip ───────────────────────────────────────────────────────────────

    /// Show a tooltip on the next `end_frame` render pass.
    pub fn tooltip(&mut self, content: impl FnOnce(&mut Ui<'_>)) {
        // Tooltip is rendered on top; collect it into a sub-UI scoped here
        // For simplicity we push a string; a full implementation would allow
        // arbitrary widget content. This version supports text only.
        let _ = content; // Caller-driven; see `set_tooltip`
    }

    pub fn set_tooltip(&mut self, text: &str) {
        self.ctx.tooltip = Some(text.to_owned());
    }

    // ── Window ────────────────────────────────────────────────────────────────

    /// Begin a window. Returns `true` if the window is visible and not collapsed.
    /// Always call `end()` regardless of the return value.
    pub fn begin(&mut self, title: &str, open: Option<&mut bool>, flags: WindowFlags) -> bool {
        crate::widgets::window::begin(self, title, open, flags)
    }

    /// End the most recently begun window.
    pub fn end(&mut self) {
        crate::widgets::window::end(self);
    }

    // ── Button ────────────────────────────────────────────────────────────────

    /// A clickable button. Returns `true` on click.
    pub fn button(&mut self, label: &str) -> bool {
        self.button_sized(label, Vec2::ZERO)
    }

    /// Button with explicit size. Use `Vec2::ZERO` for auto-size.
    pub fn button_sized(&mut self, label: &str, size: Vec2) -> bool {
        crate::widgets::button::button(self, label, size)
    }

    pub fn small_button(&mut self, label: &str) -> bool {
        crate::widgets::button::small_button(self, label)
    }

    // ── Checkbox ─────────────────────────────────────────────────────────────

    pub fn checkbox(&mut self, label: &str, v: &mut bool) -> bool {
        crate::widgets::checkbox::checkbox(self, label, v)
    }

    // ── Slider ───────────────────────────────────────────────────────────────

    pub fn slider_float(&mut self, label: &str, v: &mut f32, min: f32, max: f32) -> bool {
        crate::widgets::slider::slider_float(self, label, v, min, max)
    }

    pub fn slider_float2(&mut self, label: &str, v: &mut [f32; 2], min: f32, max: f32) -> bool {
        let mut changed = false;
        self.push_id_str("__x"); changed |= self.slider_float("X", &mut v[0], min, max); self.pop_id();
        self.same_line(4.0);
        self.push_id_str("__y"); changed |= self.slider_float("Y", &mut v[1], min, max); self.pop_id();
        self.text(label);
        changed
    }

    pub fn slider_int(&mut self, label: &str, v: &mut i32, min: i32, max: i32) -> bool {
        let mut vf = *v as f32;
        let changed = self.slider_float(label, &mut vf, min as f32, max as f32);
        if changed { *v = vf.round() as i32; }
        changed
    }

    pub fn drag_float(&mut self, label: &str, v: &mut f32, speed: f32, min: f32, max: f32) -> bool {
        crate::widgets::slider::drag_float(self, label, v, speed, min, max)
    }

    pub fn drag_int(&mut self, label: &str, v: &mut i32, speed: f32, min: i32, max: i32) -> bool {
        let mut vf = *v as f32;
        let c = self.drag_float(label, &mut vf, speed, min as f32, max as f32);
        if c { *v = vf.round() as i32; }
        c
    }

    // ── Input text ───────────────────────────────────────────────────────────

    pub fn input_text(&mut self, label: &str, buf: &mut String) -> bool {
        crate::widgets::input_text::input_text(self, label, buf, false)
    }

    pub fn input_text_multiline(&mut self, label: &str, buf: &mut String, size: Vec2) -> bool {
        let _ = size; // future: use height
        crate::widgets::input_text::input_text(self, label, buf, true)
    }

    // ── Combo ────────────────────────────────────────────────────────────────

    pub fn combo(&mut self, label: &str, selected: &mut usize, items: &[&str]) -> bool {
        crate::widgets::combo::combo(self, label, selected, items)
    }

    // ── Color edit ───────────────────────────────────────────────────────────

    pub fn color_edit3(&mut self, label: &str, color: &mut [f32; 3]) -> bool {
        let mut rgba = [color[0], color[1], color[2], 1.0];
        let c = crate::widgets::color_picker::color_edit4(self, label, &mut rgba, false);
        if c { color[0] = rgba[0]; color[1] = rgba[1]; color[2] = rgba[2]; }
        c
    }

    pub fn color_edit4(&mut self, label: &str, color: &mut [f32; 4]) -> bool {
        crate::widgets::color_picker::color_edit4(self, label, color, true)
    }

    pub fn color_picker4(&mut self, label: &str, color: &mut [f32; 4]) -> bool {
        crate::widgets::color_picker::color_picker4(self, label, color)
    }

    // ── Progress bar ─────────────────────────────────────────────────────────

    /// `fraction` in 0.0..=1.0; `size.x = 0` → full available width.
    pub fn progress_bar(&mut self, fraction: f32, size: Vec2, overlay: Option<&str>) {
        crate::widgets::progress_bar::progress_bar(self, fraction, size, overlay);
    }

    // ── Tab bar ──────────────────────────────────────────────────────────────

    pub fn begin_tab_bar(&mut self, id: &str) -> bool {
        crate::widgets::tabs::begin_tab_bar(self, id)
    }

    pub fn end_tab_bar(&mut self) {
        crate::widgets::tabs::end_tab_bar(self);
    }

    pub fn tab_item(&mut self, label: &str) -> bool {
        crate::widgets::tabs::tab_item(self, label)
    }

    pub fn end_tab_item(&mut self) {
        crate::widgets::tabs::end_tab_item(self);
    }

    // ─── Collapsing header ────────────────────────────────────────────────────

    pub fn collapsing(&mut self, label: &str) -> bool {
        crate::widgets::button::collapsing_header(self, label)
    }

    // ── Image ─────────────────────────────────────────────────────────────────

    pub fn image(&mut self, texture: TextureId, size: Vec2) {
        let sp  = self.ctx.style.item_spacing;
        if let Some(l) = self.ctx.current_layout_mut() {
            let pos = l.place(size, sp);
            drop(l); // borrow ends
            let draw = &mut self.ctx.draw_list;
            draw.image_quad(texture, pos, pos + size, Vec2::ZERO, Vec2::ONE, Color::WHITE);
        }
    }

    // ─── Internal helpers ──────────────────────────────────────────────────────

    /// Place a text-sized item in the current layout; returns the position.
    pub(crate) fn layout_next_for_text(&mut self, text: &str) -> Option<Vec2> {
        let fs  = self.ctx.style.font_size * self.scale;
        let w   = self.font.measure(text, fs);
        let sz  = Vec2::new(w, fs);
        let sp  = self.ctx.style.item_spacing;
        self.ctx.current_layout_mut().map(|l| l.place(sz, sp))
    }

    /// Place an item of explicit size; returns its top-left position.
    pub(crate) fn layout_next(&mut self, size: Vec2) -> Option<Vec2> {
        let sp = self.ctx.style.item_spacing;
        self.ctx.current_layout_mut().map(|l| l.place(size, sp))
    }

    /// Emit text glyphs into the draw list at `pos`.
    pub(crate) fn draw_text(&mut self, text: &str, pos: Vec2, col: Color) {
        let fs = self.ctx.style.font_size * self.scale;
        let mut x = pos.x;
        let mut glyphs: Vec<(Vec2, Vec2, Vec2, Vec2)> = Vec::with_capacity(text.len());
        for ch in text.chars() {
            if let Some(g) = self.font.glyph(ch, fs) {
                let p_min = Vec2::new(x, pos.y + g.offset_y);
                let p_max = p_min + g.size;
                glyphs.push((p_min, p_max, g.uv_min, g.uv_max));
                x += g.advance_x;
            } else if ch == ' ' {
                x += fs * 0.25;
            }
        }
        self.ctx.draw_list.push_texture(TextureId::FONT);
        self.ctx.draw_list.add_text_raw(&glyphs, col);
        self.ctx.draw_list.pop_texture();
    }

    /// Measure text width in logical pixels.
    pub(crate) fn text_width(&self, text: &str) -> f32 {
        let fs = self.ctx.style.font_size * self.scale;
        self.font.measure(text, fs)
    }
}

// ─── Tokens ──────────────────────────────────────────────────────────────────

pub struct StyleColorToken {
    pub(crate) var: StyleColor,
    pub(crate) old: Color,
}

pub struct StyleVarToken(pub(crate) crate::style::StyleVarRestore);

// ─── Context extension: frame builder ────────────────────────────────────────

impl Context {
    /// Run an immediate-mode frame, borrowing `self` and the font atlas.
    ///
    /// ```rust,ignore
    /// ctx.frame(&*my_font, 1.0, |ui| {
    ///     if ui.begin("Demo", &mut open, WindowFlags::empty()) {
    ///         ui.text("Hello!");
    ///     }
    ///     ui.end();
    /// });
    /// ```
    pub fn frame<F>(&mut self, font: &dyn FontAtlas, scale: f32, f: F)
    where
        F: FnOnce(&mut Ui<'_>),
    {
        self.new_frame();
        let mut ui = Ui::new(self, font, scale);
        f(&mut ui);
    }
}
