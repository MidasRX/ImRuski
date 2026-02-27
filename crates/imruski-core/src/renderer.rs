//! Renderer trait + font atlas trait.

use crate::draw_list::{DrawList, TextureId};
use crate::Vec2;

/// A rendered frame ready to hand to a backend.
pub struct RenderFrame<'a> {
    pub draw_list:    &'a DrawList,
    pub display_size: Vec2,
    pub scale_factor: f32,
}

/// Font glyph information returned by [`FontAtlas`].
#[derive(Debug, Clone, Copy)]
pub struct GlyphInfo {
    /// Top-left UV in the atlas texture.
    pub uv_min: Vec2,
    /// Bottom-right UV in the atlas texture.
    pub uv_max: Vec2,
    /// Rendered size in pixels.
    pub size: Vec2,
    /// Horizontal advance width in pixels.
    pub advance_x: f32,
    /// Vertical offset from baseline.
    pub offset_y: f32,
}

/// Trait for font atlas providers.
///
/// Each backend can supply its own atlas (bitmap, vector, etc.).
pub trait FontAtlas: Send + Sync {
    /// Glyph lookup. Returns `None` for unsupported characters.
    fn glyph(&self, ch: char, size_px: f32) -> Option<GlyphInfo>;
    /// Atlas texture handle (already uploaded to the GPU by the backend).
    fn texture(&self) -> TextureId;
    /// Measure the advance width of a string.
    fn measure(&self, text: &str, size_px: f32) -> f32 {
        text.chars()
            .filter_map(|c| self.glyph(c, size_px))
            .map(|g| g.advance_x)
            .sum()
    }
}

/// Core renderer interface – implement this for every rendering backend.
///
/// # Call order per frame
///
/// ```text
/// renderer.begin_frame()
/// // … widget computation fills a DrawList …
/// renderer.render(frame)
/// renderer.end_frame()
/// ```
pub trait Renderer: Send {
    // ── frame lifecycle ───────────────────────────────────────────────────────

    /// Called once at the start of every frame (acquire surface, reset state).
    fn begin_frame(&mut self);

    /// Submit the accumulated draw list to the GPU / surface.
    fn render(&mut self, frame: RenderFrame<'_>);

    /// Present the frame (swap buffers / end draw call).
    fn end_frame(&mut self);

    // ── resource management ───────────────────────────────────────────────────

    /// Upload a raw RGBA (4 bytes per pixel) bitmap and return a texture handle.
    fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> TextureId;

    /// Release a previously created texture.
    fn destroy_texture(&mut self, id: TextureId);

    // ── query ─────────────────────────────────────────────────────────────────

    /// Display size in logical pixels.
    fn display_size(&self) -> Vec2;

    /// Scaling factor (DPI).
    fn scale_factor(&self) -> f32 { 1.0 }

    /// Font atlas used for text rendering.
    fn font_atlas(&self) -> &dyn FontAtlas;
}
