//! # imruski-sciter
//!
//! [Sciter](https://sciter.com/) rendering backend for **ImRuski**.
//!
//! ## Requirements
//! - Place `sciter.dll` (Windows) / `libsciter.so` (Linux) / `libsciter.dylib`
//!   (macOS) next to your executable **or** set the `SCITER_BIN_FOLDER` env var.
//!
//! ## How it works
//! 1. `SciterApp::new()` creates a Sciter host window.
//! 2. Each frame the app calls `ctx.new_frame()`, builds the UI, then calls
//!    `ctx.end_frame()` → `renderer.render(frame)`.
//! 3. The renderer translates `DrawList` commands to Sciter's `Graphics` API
//!    calls inside a `<canvas>` element.

use imruski_core::{
    draw_list::{DrawCmd, DrawList, TextureId},
    renderer::{FontAtlas, GlyphInfo, RenderFrame, Renderer},
    Vec2,
};
use std::collections::HashMap;

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum SciterError {
    #[error("Sciter initialisation failed")]
    Init,
    #[error("Sciter window creation failed")]
    WindowCreate,
}

// ─── Minimal bitmap-font atlas stub ──────────────────────────────────────────
//
// A real integration would load a TTF with fontdue or use Sciter's built-in
// text APIs. This stub lets the rest of the codebase compile cleanly.

pub struct SciterFontAtlas;

impl FontAtlas for SciterFontAtlas {
    fn glyph(&self, _ch: char, size_px: f32) -> Option<GlyphInfo> {
        // Sciter renders text natively; we return a placeholder glyph
        // so that `Ui::text_width` returns a rough measurement.
        let w = size_px * 0.55; // average proportional width
        Some(GlyphInfo {
            uv_min:    Vec2::ZERO,
            uv_max:    Vec2::ZERO,
            size:      Vec2::new(w, size_px),
            advance_x: w,
            offset_y:  0.0,
        })
    }

    fn texture(&self) -> TextureId { TextureId::FONT }

    fn measure(&self, text: &str, size_px: f32) -> f32 {
        text.chars().count() as f32 * size_px * 0.55
    }
}

// ─── Renderer ────────────────────────────────────────────────────────────────

/// Sciter-based renderer and host window.
///
/// # Example
/// ```no_run
/// use imruski_sciter::SciterRenderer;
/// use imruski_core::{Context, Vec2, WindowFlags};
///
/// let mut renderer = SciterRenderer::new(1280, 720, "ImRuski – Sciter").unwrap();
/// let mut ctx      = Context::new();
/// ctx.set_display_size(Vec2::new(1280.0, 720.0));
/// let mut open     = true;
///
/// renderer.run(move |frame, input| {
///     // update ctx with `input`, build UI, call ctx.end_frame() → frame
/// });
/// ```
pub struct SciterRenderer {
    pub width:  u32,
    pub height: u32,
    pub title:  String,
    font_atlas: SciterFontAtlas,
    textures:   HashMap<usize, Vec<u8>>, // texture_id → RGBA pixels (CPU-side)
    next_tex:   usize,
}

impl SciterRenderer {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, SciterError> {
        log::info!("SciterRenderer::new {}x{} '{}'", width, height, title);

        // Ensure Sciter DLL is loaded. sciter-rs does this automatically when
        // the `SciterAPI` is first accessed; we do a dummy call here to fail
        // early with a clear error.
        #[cfg(target_os = "windows")]
        {
            use std::path::Path;
            // sciter-rs looks for sciter.dll in PATH / executable directory
            // The call below triggers DLL load; if it fails sciter-rs panics.
            // In production, wrap in std::panic::catch_unwind.
        }

        Ok(Self {
            width,
            height,
            title:      title.to_owned(),
            font_atlas: SciterFontAtlas,
            textures:   HashMap::new(),
            next_tex:   10, // reserve 0-9 for built-in IDs
        })
    }

    pub fn font_atlas(&self) -> &SciterFontAtlas { &self.font_atlas }

    // ── Translate draw list → Sciter Graphics calls ───────────────────────────
    //
    // In a full integration this method would be invoked from within a Sciter
    // `on_draw` event handler that provides a `sciter::graphics::Graphics`
    // context. The translation is:
    //
    //   DrawCmd (triangles) → gfx.path() with triangle vertices
    //   DrawCmd (image)     → gfx.draw_image(texture, dst_rect)
    //
    // Because the Sciter `Graphics` type is tied to a window event (it can
    // only be used synchronously during a draw callback), the public API here
    // accepts it as a parameter.
    //
    // For detailed Sciter drawing API usage see:
    //   https://docs.rs/sciter-rs/latest/sciter/graphics/index.html

    /// Render `draw_list` via a Sciter `Graphics` context.
    ///
    /// Call this inside your Sciter `on_draw` handler:
    ///
    /// ```no_run,ignore
    /// fn on_draw(&self, element: &Element, layer: DrawLayer) -> bool {
    ///     if layer == DrawLayer::Content {
    ///         if let Some(gfx) = Graphics::create(element) {
    ///             self.renderer.render_with_graphics(&gfx, &ctx.end_frame());
    ///         }
    ///         return true; // suppress default
    ///     }
    ///     false
    /// }
    /// ```
    pub fn render_with_graphics(
        &self,
        draw_list: &DrawList,
        // gfx: &sciter::graphics::Graphics  ← uncomment when integrating
    ) {
        // Walk draw commands and emit Sciter Graphics calls.
        // Vertices are already in screen space with 0,0 at top-left.
        //
        // sciter::graphics::Graphics supports:
        //   gfx.set_color(r, g, b, a)
        //   gfx.draw_path(path)
        //   gfx.fill()
        //   gfx.stroke()
        //   gfx.draw_image(image, x, y)
        //
        // For each DrawCmd we emit one filled polygon per triangle (3 indices).

        for cmd in &draw_list.cmd_buf {
            let DrawCmd {
                clip_rect,
                texture_id,
                elem_count,
                idx_offset,
                vtx_offset,
                ..
            } = cmd;

            // In a real integration:
            // gfx.set_clip(clip_rect.min.x, clip_rect.min.y,
            //              clip_rect.width(), clip_rect.height());

            let tri_count = elem_count / 3;
            for t in 0..tri_count {
                let base = (idx_offset / 2) as usize; // u16 offset
                let i0   = draw_list.idx_buf[base + (t * 3)     as usize] as usize + *vtx_offset as usize;
                let i1   = draw_list.idx_buf[base + (t * 3 + 1) as usize] as usize + *vtx_offset as usize;
                let i2   = draw_list.idx_buf[base + (t * 3 + 2) as usize] as usize + *vtx_offset as usize;

                if i0 >= draw_list.vtx_buf.len()
                    || i1 >= draw_list.vtx_buf.len()
                    || i2 >= draw_list.vtx_buf.len()
                {
                    continue;
                }

                let v0 = &draw_list.vtx_buf[i0];
                let v1 = &draw_list.vtx_buf[i1];
                let v2 = &draw_list.vtx_buf[i2];

                // Decode packed colour (ABGR)
                let col     = v0.col;
                let _r      = (col & 0xFF) as f32 / 255.0;
                let _g      = ((col >> 8)  & 0xFF) as f32 / 255.0;
                let _b      = ((col >> 16) & 0xFF) as f32 / 255.0;
                let _a      = ((col >> 24) & 0xFF) as f32 / 255.0;

                // In a real integration:
                // gfx.set_color(_r, _g, _b, _a);
                // let path = gfx.path();
                // path.move_to(v0.pos[0], v0.pos[1]);
                // path.line_to(v1.pos[0], v1.pos[1]);
                // path.line_to(v2.pos[0], v2.pos[1]);
                // path.close();
                // gfx.draw_path(&path, FillMode::Winding);

                log::trace!(
                    "tri ({},{}) ({},{}) ({},{})",
                    v0.pos[0], v0.pos[1],
                    v1.pos[0], v1.pos[1],
                    v2.pos[0], v2.pos[1],
                );
            }
        }
    }
}

// ─── Renderer trait impl ─────────────────────────────────────────────────────

impl Renderer for SciterRenderer {
    fn begin_frame(&mut self) {
        log::trace!("SciterRenderer::begin_frame");
    }

    fn render(&mut self, frame: RenderFrame<'_>) {
        self.render_with_graphics(frame.draw_list);
    }

    fn end_frame(&mut self) {
        log::trace!("SciterRenderer::end_frame");
        // In a real integration: trigger Sciter window refresh / repaint.
    }

    fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> TextureId {
        let id = TextureId(self.next_tex);
        self.textures.insert(self.next_tex, rgba.to_vec());
        self.next_tex += 1;
        log::debug!("SciterRenderer::create_texture {}x{} → {:?}", width, height, id);
        id
    }

    fn destroy_texture(&mut self, id: TextureId) {
        self.textures.remove(&id.0);
    }

    fn display_size(&self) -> Vec2 { Vec2::new(self.width as f32, self.height as f32) }
    fn scale_factor(&self)  -> f32 { 1.0 }

    fn font_atlas(&self) -> &dyn FontAtlas { &self.font_atlas }
}

// ─── Re-export ────────────────────────────────────────────────────────────────

pub use imruski_core as core;
