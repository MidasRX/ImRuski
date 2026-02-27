//! Draw-command accumulator – the bridge between the widget layer and renderers.
//!
//! The widget layer emits high-level draw calls; the renderer back-end
//! converts them to GPU primitives.

use crate::{Color, Rect, Vec2};

// ─── Vertex ──────────────────────────────────────────────────────────────────

/// A single render vertex with position, UV, and a packed RGBA colour.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct DrawVert {
    /// Screen-space position.
    pub pos: [f32; 2],
    /// Normalised texture coordinates.
    pub uv:  [f32; 2],
    /// 0xAABBGGRR packed colour.
    pub col: u32,
}

// SAFETY: all fields are plain scalar types with no padding surprises.
unsafe impl bytemuck::Pod      for DrawVert {}
unsafe impl bytemuck::Zeroable for DrawVert {}

/// 16-bit index type – matches ImGui default.
pub type DrawIdx = u16;

// ─── TextureId ───────────────────────────────────────────────────────────────

/// Opaque handle to a GPU texture / atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TextureId(pub usize);

impl TextureId {
    pub const FONT: Self = Self(1);
    pub const WHITE: Self = Self(0);
}

// ─── DrawCmd ─────────────────────────────────────────────────────────────────

/// A single batched render command.
#[derive(Debug, Clone)]
pub struct DrawCmd {
    /// Scissor / clip rectangle in screen pixels.
    pub clip_rect: Rect,
    pub texture_id: TextureId,
    /// Number of indices to draw (always a multiple of 3).
    pub elem_count: u32,
    /// Byte offset into the index buffer for this draw call.
    pub idx_offset: u32,
    /// Value added to each index.
    pub vtx_offset: u32,
}

// ─── DrawList ────────────────────────────────────────────────────────────────

/// CPU-side vertex / index / command buffer.
///
/// Cleared at the beginning of every frame.
#[derive(Debug, Default)]
pub struct DrawList {
    pub vtx_buf: Vec<DrawVert>,
    pub idx_buf: Vec<DrawIdx>,
    pub cmd_buf: Vec<DrawCmd>,

    // Bookkeeping
    clip_stack: Vec<Rect>,
    tex_stack:  Vec<TextureId>,
    vtx_start:  u32,
    idx_start:  u32,
}

impl DrawList {
    pub fn clear(&mut self) {
        self.vtx_buf.clear();
        self.idx_buf.clear();
        self.cmd_buf.clear();
        self.clip_stack.clear();
        self.tex_stack.clear();
        self.vtx_start = 0;
        self.idx_start = 0;
    }

    // ─── clip stack ──────────────────────────────────────────────────────────

    pub fn push_clip_rect(&mut self, rect: Rect) {
        // Intersect with parent clip
        let clip = if let Some(&parent) = self.clip_stack.last() {
            rect.intersect(parent)
        } else {
            rect
        };
        self.clip_stack.push(clip);
        self.add_draw_cmd();
    }

    pub fn pop_clip_rect(&mut self) {
        self.clip_stack.pop();
        self.add_draw_cmd();
    }

    pub fn clip_rect(&self) -> Option<Rect> { self.clip_stack.last().copied() }

    // ─── texture stack ───────────────────────────────────────────────────────

    pub fn push_texture(&mut self, id: TextureId) {
        self.tex_stack.push(id);
        self.add_draw_cmd();
    }

    pub fn pop_texture(&mut self) {
        self.tex_stack.pop();
        self.add_draw_cmd();
    }

    // ─── internal command management ─────────────────────────────────────────

    fn add_draw_cmd(&mut self) {
        let clip = self.clip_stack.last().copied().unwrap_or(Rect {
            min: Vec2::ZERO,
            max: Vec2::splat(f32::MAX),
        });
        let tex = self.tex_stack.last().copied().unwrap_or(TextureId::WHITE);
        let vtx_off = self.vtx_buf.len() as u32;
        let idx_off = self.idx_buf.len() as u32;
        self.cmd_buf.push(DrawCmd {
            clip_rect: clip,
            texture_id: tex,
            elem_count: 0,
            idx_offset: idx_off,
            vtx_offset: vtx_off,
        });
        self.vtx_start = vtx_off;
        self.idx_start = idx_off;
    }

    fn current_cmd_mut(&mut self) -> &mut DrawCmd {
        if self.cmd_buf.is_empty() { self.add_draw_cmd(); }
        self.cmd_buf.last_mut().unwrap()
    }

    // ─── raw primitive helpers ───────────────────────────────────────────────

    fn add_vert(&mut self, pos: Vec2, uv: Vec2, col: u32) {
        self.vtx_buf.push(DrawVert { pos: pos.into(), uv: uv.into(), col });
    }

    fn add_idx(&mut self, base: u32, a: u32, b: u32, c: u32) {
        let base = (base - self.vtx_start) as DrawIdx;
        let (a, b, c) = (
            base + a as DrawIdx,
            base + b as DrawIdx,
            base + c as DrawIdx,
        );
        self.idx_buf.extend_from_slice(&[a, b, c]);
        self.current_cmd_mut().elem_count += 3;
    }

    // ─── Filled primitives ───────────────────────────────────────────────────

    /// Solid filled rectangle.
    pub fn filled_rect(&mut self, rect: Rect, rounding: f32, col: Color) {
        if rect.is_empty() { return; }
        let c = col.to_rgba_u32();
        if rounding < 0.5 {
            self.fill_rect_raw(rect, c);
        } else {
            self.fill_rounded_rect(rect, rounding, c);
        }
    }

    fn fill_rect_raw(&mut self, r: Rect, col: u32) {
        let base = self.vtx_buf.len() as u32;
        let uv = Vec2::new(0.0, 0.0); // white pixel UV
        self.add_vert(r.min,                              uv, col);
        self.add_vert(Vec2::new(r.max.x, r.min.y),       uv, col);
        self.add_vert(r.max,                              uv, col);
        self.add_vert(Vec2::new(r.min.x, r.max.y),       uv, col);
        self.add_idx(base, 0, 1, 2);
        self.add_idx(base, 0, 2, 3);
    }

    fn fill_rounded_rect(&mut self, r: Rect, rounding: f32, col: u32) {
        // Clamp rounding
        let rounding = rounding.min(r.width() * 0.5).min(r.height() * 0.5);
        // Corner segments
        const SEGS: usize = 8;
        let corners = [
            Vec2::new(r.min.x + rounding, r.min.y + rounding),
            Vec2::new(r.max.x - rounding, r.min.y + rounding),
            Vec2::new(r.max.x - rounding, r.max.y - rounding),
            Vec2::new(r.min.x + rounding, r.max.y - rounding),
        ];
        let angles = [
            (std::f32::consts::PI, 1.5 * std::f32::consts::PI),
            (1.5 * std::f32::consts::PI, 2.0 * std::f32::consts::PI),
            (0.0, 0.5 * std::f32::consts::PI),
            (0.5 * std::f32::consts::PI, std::f32::consts::PI),
        ];
        let uv = Vec2::ZERO;
        let center_base = self.vtx_buf.len() as u32;
        let center = r.center();
        self.add_vert(center, uv, col);

        for (ci, (start, end)) in angles.iter().enumerate() {
            for s in 0..=SEGS {
                let t = start + (end - start) * (s as f32 / SEGS as f32);
                let pt = corners[ci] + Vec2::new(t.cos(), t.sin()) * rounding;
                self.add_vert(pt, uv, col);
            }
        }
        let total = (SEGS + 1) * 4;
        for i in 0..total {
            let a = (i + 1) as u32;
            let b = (i + 2) as u32;
            self.add_idx(center_base, 0, a, if b > total as u32 { 1 } else { b });
        }
    }

    /// Outlined rectangle (4 quads).
    pub fn rect_outline(&mut self, rect: Rect, thickness: f32, col: Color) {
        let c = col.to_rgba_u32();
        let t = thickness;
        // top
        self.fill_rect_raw(Rect::from_min_size(rect.min, Vec2::new(rect.width(), t)), c);
        // bottom
        self.fill_rect_raw(Rect::from_min_size(Vec2::new(rect.min.x, rect.max.y - t), Vec2::new(rect.width(), t)), c);
        // left
        self.fill_rect_raw(Rect::from_min_size(rect.min, Vec2::new(t, rect.height())), c);
        // right
        self.fill_rect_raw(Rect::from_min_size(Vec2::new(rect.max.x - t, rect.min.y), Vec2::new(t, rect.height())), c);
    }

    /// A horizontal/vertical line segment rendered as a quad.
    pub fn line(&mut self, a: Vec2, b: Vec2, thickness: f32, col: Color) {
        let c = col.to_rgba_u32();
        let d = b - a;
        let len = d.length();
        if len < 0.01 { return; }
        let n = Vec2::new(-d.y, d.x) * (thickness * 0.5 / len);
        let uv = Vec2::ZERO;
        let base = self.vtx_buf.len() as u32;
        self.add_vert(a + n, uv, c);
        self.add_vert(a - n, uv, c);
        self.add_vert(b - n, uv, c);
        self.add_vert(b + n, uv, c);
        self.add_idx(base, 0, 1, 2);
        self.add_idx(base, 0, 2, 3);
    }

    /// Filled circle approximation (N-gon).
    pub fn filled_circle(&mut self, center: Vec2, radius: f32, col: Color, segments: usize) {
        let c = col.to_rgba_u32();
        let uv = Vec2::ZERO;
        let segs = segments.max(6);
        let base = self.vtx_buf.len() as u32;
        self.add_vert(center, uv, c);
        for i in 0..=segs {
            let a = 2.0 * std::f32::consts::PI * i as f32 / segs as f32;
            let pt = center + Vec2::new(a.cos(), a.sin()) * radius;
            self.add_vert(pt, uv, c);
        }
        for i in 0..segs as u32 {
            self.add_idx(base, 0, i + 1, i + 2);
        }
    }

    /// Render a textured quad (e.g. an image or font glyph).
    pub fn image_quad(
        &mut self,
        texture: TextureId,
        p_min: Vec2, p_max: Vec2,
        uv_min: Vec2, uv_max: Vec2,
        col: Color,
    ) {
        let c    = col.to_rgba_u32();
        let base = self.vtx_buf.len() as u32;
        self.push_texture(texture);
        self.add_vert(p_min,                             uv_min,                         c);
        self.add_vert(Vec2::new(p_max.x, p_min.y),      Vec2::new(uv_max.x, uv_min.y), c);
        self.add_vert(p_max,                             uv_max,                         c);
        self.add_vert(Vec2::new(p_min.x, p_max.y),      Vec2::new(uv_min.x, uv_max.y), c);
        self.add_idx(base, 0, 1, 2);
        self.add_idx(base, 0, 2, 3);
        self.pop_texture();
    }

    /// Simple text rendering using the font texture atlas.
    /// Glyph UV lookup is delegated to the renderer via a pre-baked layout.
    /// Here we emit one pre-built glyph quad per character.
    pub fn add_text_raw(
        &mut self,
        glyphs: &[(Vec2, Vec2, Vec2, Vec2)], // (pos_min, pos_max, uv_min, uv_max)
        col: Color,
    ) {
        let c = col.to_rgba_u32();
        for &(p_min, p_max, uv_min, uv_max) in glyphs {
            let base = self.vtx_buf.len() as u32;
            self.add_vert(p_min,                             uv_min,                        c);
            self.add_vert(Vec2::new(p_max.x, p_min.y),      Vec2::new(uv_max.x, uv_min.y), c);
            self.add_vert(p_max,                             uv_max,                        c);
            self.add_vert(Vec2::new(p_min.x, p_max.y),      Vec2::new(uv_min.x, uv_max.y), c);
            self.add_idx(base, 0, 1, 2);
            self.add_idx(base, 0, 2, 3);
        }
    }

    // ─── Triangle ─────────────────────────────────────────────────────────────

    pub fn triangle_filled(&mut self, a: Vec2, b: Vec2, c_pt: Vec2, col: Color) {
        let c = col.to_rgba_u32();
        let uv = Vec2::ZERO;
        let base = self.vtx_buf.len() as u32;
        self.add_vert(a,    uv, c);
        self.add_vert(b,    uv, c);
        self.add_vert(c_pt, uv, c);
        self.add_idx(base, 0, 1, 2);
    }
}
