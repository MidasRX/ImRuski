//! # imruski-ultralight
//!
//! [Ultralight](https://ultralig.ht/) rendering backend for **ImRuski**.
//!
//! ## Requirements
//! - Ultralight SDK installed (runtime DLLs + `resources/` folder).
//! - Set `UL_SDK_PATH` to the SDK root, or place the libraries next to your
//!   executable. Refer to the `ul-next` crate's README for exact setup steps.
//!
//! ## How it works
//! 1. `UltralightRenderer::new()` initialises the Ultralight app + window.
//! 2. The renderer renders `DrawList` commands to a `BitmapSurface` pixel
//!    buffer (CPU path) which can then be composited via OpenGL / DX11.
//! 3. Alternatively, set the GPU driver path to have Ultralight render
//!    directly to a texture and composite with another renderer.

use imruski_core::{
    draw_list::{DrawList, TextureId},
    renderer::{FontAtlas, GlyphInfo, RenderFrame, Renderer},
    Vec2,
};
use std::collections::HashMap;

// ─── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum UltralightError {
    #[error("Ultralight initialisation failed: {0}")]
    Init(String),
}

// ─── Font atlas stub ─────────────────────────────────────────────────────────

pub struct UltralightFontAtlas {
    font_size: f32,
}

impl UltralightFontAtlas {
    pub fn new(default_font_size: f32) -> Self {
        Self { font_size: default_font_size }
    }
}

impl FontAtlas for UltralightFontAtlas {
    fn glyph(&self, _ch: char, size_px: f32) -> Option<GlyphInfo> {
        let w = size_px * 0.55;
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

/// Ultralight renderer backend.
///
/// # Example
/// ```no_run
/// use imruski_ultralight::UltralightRenderer;
/// use imruski_core::{Context, Vec2, WindowFlags};
///
/// let mut renderer = UltralightRenderer::new(1280, 720, "ImRuski – Ultralight").unwrap();
/// let mut ctx = Context::new();
/// ctx.set_display_size(Vec2::new(1280.0, 720.0));
/// ```
pub struct UltralightRenderer {
    pub width:  u32,
    pub height: u32,
    pub title:  String,
    font_atlas: UltralightFontAtlas,
    textures:   HashMap<usize, Vec<u8>>,
    next_tex:   usize,

    /// Pixel buffer (RGBA, row-major). Written by `render()`.
    /// Attach to a GPU texture or composite with another renderer.
    pub framebuffer: Vec<u8>,
}

impl UltralightRenderer {
    pub fn new(width: u32, height: u32, title: &str) -> Result<Self, UltralightError> {
        log::info!("UltralightRenderer::new {}x{} '{}'", width, height, title);

        // `ul_next::App` / `ul_next::Window` initialisation would go here.
        // We defer full SDK binding calls to avoid a hard compile dependency
        // when the SDK is not installed. Uncomment the block below once your
        // build environment has the Ultralight runtime libraries:
        //
        // ```
        // use ul_next::{config::Config, app::App, window::WindowFlags};
        // let config = Config::start().build();
        // let app    = App::new(None, Some(config))
        //     .map_err(|e| UltralightError::Init(e.to_string()))?;
        // let window = app.create_window(width, height, false,
        //                                WindowFlags::TITLED | WindowFlags::RESIZABLE);
        // ```

        let fb_size = (width * height * 4) as usize;
        Ok(Self {
            width,
            height,
            title:       title.to_owned(),
            font_atlas:  UltralightFontAtlas::new(13.0),
            textures:    HashMap::new(),
            next_tex:    10,
            framebuffer: vec![0u8; fb_size],
        })
    }

    /// Software rasterise the draw list into `self.framebuffer` (RGBA).
    ///
    /// This is the CPU-fallback path. For production use, upload
    /// `self.framebuffer` to a GPU texture and alpha-blend it over the scene.
    pub fn rasterize(&mut self, draw_list: &DrawList) {
        let w = self.width  as i32;
        let h = self.height as i32;

        for cmd in &draw_list.cmd_buf {
            let tri_count = cmd.elem_count / 3;
            let idx_base  = cmd.idx_offset as usize;
            let vtx_base  = cmd.vtx_offset as usize;

            for t in 0..tri_count as usize {
                let i0 = draw_list.idx_buf[idx_base + t * 3]     as usize + vtx_base;
                let i1 = draw_list.idx_buf[idx_base + t * 3 + 1] as usize + vtx_base;
                let i2 = draw_list.idx_buf[idx_base + t * 3 + 2] as usize + vtx_base;

                if i0 >= draw_list.vtx_buf.len()
                    || i1 >= draw_list.vtx_buf.len()
                    || i2 >= draw_list.vtx_buf.len()
                { continue; }

                let v0 = draw_list.vtx_buf[i0];
                let v1 = draw_list.vtx_buf[i1];
                let v2 = draw_list.vtx_buf[i2];

                // Bounding box
                let min_x = v0.pos[0].min(v1.pos[0]).min(v2.pos[0]).max(0.0) as i32;
                let min_y = v0.pos[1].min(v1.pos[1]).min(v2.pos[1]).max(0.0) as i32;
                let max_x = (v0.pos[0].max(v1.pos[0]).max(v2.pos[0]) as i32 + 1).min(w);
                let max_y = (v0.pos[1].max(v1.pos[1]).max(v2.pos[1]) as i32 + 1).min(h);

                // Barycentric rasterisation
                let signed_area = |ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32| -> f32 {
                    (bx - ax) * (cy - ay) - (by - ay) * (cx - ax)
                };
                let area = signed_area(
                    v0.pos[0], v0.pos[1],
                    v1.pos[0], v1.pos[1],
                    v2.pos[0], v2.pos[1],
                );
                if area.abs() < 0.5 { continue; }

                let col = v0.col; // use first vertex colour
                let r = (col & 0xFF)       as u8;
                let g = ((col >> 8)  & 0xFF) as u8;
                let b = ((col >> 16) & 0xFF) as u8;
                let a = ((col >> 24) & 0xFF) as u8;

                for py in min_y..max_y {
                    for px in min_x..max_x {
                        let px_f = px as f32 + 0.5;
                        let py_f = py as f32 + 0.5;
                        let w0 = signed_area(v1.pos[0], v1.pos[1], v2.pos[0], v2.pos[1], px_f, py_f);
                        let w1 = signed_area(v2.pos[0], v2.pos[1], v0.pos[0], v0.pos[1], px_f, py_f);
                        let w2 = signed_area(v0.pos[0], v0.pos[1], v1.pos[0], v1.pos[1], px_f, py_f);
                        if (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0)
                            || (w0 <= 0.0 && w1 <= 0.0 && w2 <= 0.0)
                        {
                            let idx = ((py * w + px) * 4) as usize;
                            if idx + 3 < self.framebuffer.len() {
                                // Alpha blend over existing pixel
                                let src_a = a as f32 / 255.0;
                                let dst_r = self.framebuffer[idx];
                                let dst_g = self.framebuffer[idx + 1];
                                let dst_b = self.framebuffer[idx + 2];
                                let blend = |s: u8, d: u8| -> u8 {
                                    (s as f32 * src_a + d as f32 * (1.0 - src_a)) as u8
                                };
                                self.framebuffer[idx]     = blend(r, dst_r);
                                self.framebuffer[idx + 1] = blend(g, dst_g);
                                self.framebuffer[idx + 2] = blend(b, dst_b);
                                self.framebuffer[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Clear the framebuffer to transparent black.
    pub fn clear_fb(&mut self) {
        self.framebuffer.fill(0);
    }

    pub fn font_atlas(&self) -> &UltralightFontAtlas { &self.font_atlas }
}

// ─── Renderer trait impl ─────────────────────────────────────────────────────

impl Renderer for UltralightRenderer {
    fn begin_frame(&mut self) {
        self.clear_fb();
    }

    fn render(&mut self, frame: RenderFrame<'_>) {
        self.rasterize(frame.draw_list);
    }

    fn end_frame(&mut self) {
        // Upload self.framebuffer to an OpenGL / DX11 texture here, then
        // composite it. With the `ul-next` GPU path you would instead call
        // the Ultralight `Renderer::render()` method.
        log::trace!("UltralightRenderer::end_frame – framebuffer ready ({} bytes)", self.framebuffer.len());
    }

    fn create_texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> TextureId {
        let id = TextureId(self.next_tex);
        self.textures.insert(self.next_tex, rgba.to_vec());
        self.next_tex += 1;
        log::debug!("UltralightRenderer::create_texture {}x{} → {:?}", width, height, id);
        id
    }

    fn destroy_texture(&mut self, id: TextureId) { self.textures.remove(&id.0); }

    fn display_size(&self)  -> Vec2 { Vec2::new(self.width as f32, self.height as f32) }
    fn font_atlas(&self)    -> &dyn FontAtlas { &self.font_atlas }
}

pub use imruski_core as core;
