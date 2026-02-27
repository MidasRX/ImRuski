//! # imruski-core
//!
//! Backend-agnostic immediate-mode GUI engine.
//! Pair with one of the backend crates to get a rendered window:
//! - `imruski-sciter`      (Sciter HTML engine)
//! - `imruski-ultralight`  (Ultralight GPU web renderer)
//! - `imruski-dx11`        (DirectX 11 game-overlay hook)

pub mod context;
pub mod draw_list;
pub mod id;
pub mod input;
pub mod layout;
pub mod renderer;
pub mod style;
pub mod ui;
pub mod widgets;

// ─── re-exports ──────────────────────────────────────────────────────────────
pub use context::Context;
pub use draw_list::{DrawCmd, DrawList, DrawVert, TextureId};
pub use id::Id;
pub use input::{InputState, Key, Modifiers, MouseButton};
pub use layout::LayoutDir;
pub use renderer::{Renderer, RenderFrame};
pub use style::{Style, StyleColor, StyleVar};
pub use ui::Ui;

// ─── Prelude ─────────────────────────────────────────────────────────────────
pub mod prelude {
    pub use super::{
        context::Context,
        draw_list::TextureId,
        id::Id,
        input::{Key, Modifiers, MouseButton},
        renderer::Renderer,
        style::StyleColor,
        ui::Ui,
        Color, Rect, Vec2, WindowFlags,
    };
}

// ─── Primitive math types ─────────────────────────────────────────────────── //

/// 2-D floating-point vector.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };

    #[inline] pub fn new(x: f32, y: f32) -> Self { Self { x, y } }
    #[inline] pub fn splat(v: f32) -> Self { Self { x: v, y: v } }

    #[inline] pub fn length(self) -> f32 { (self.x * self.x + self.y * self.y).sqrt() }
    #[inline] pub fn dot(self, o: Self) -> f32 { self.x * o.x + self.y * o.y }

    #[inline] pub fn floor(self) -> Self { Self::new(self.x.floor(), self.y.floor()) }
    #[inline] pub fn ceil(self)  -> Self { Self::new(self.x.ceil(),  self.y.ceil())  }

    #[inline] pub fn min(self, o: Self) -> Self { Self::new(self.x.min(o.x), self.y.min(o.y)) }
    #[inline] pub fn max(self, o: Self) -> Self { Self::new(self.x.max(o.x), self.y.max(o.y)) }

    #[inline]
    pub fn clamp(self, lo: Self, hi: Self) -> Self {
        Self::new(self.x.clamp(lo.x, hi.x), self.y.clamp(lo.y, hi.y))
    }

    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    #[inline]
    pub fn rotate(self, angle: f32) -> Self {
        let (s, c) = angle.sin_cos();
        Self::new(self.x * c - self.y * s, self.x * s + self.y * c)
    }
}

impl std::ops::Add    for Vec2 { type Output = Self; fn add(self, r: Self) -> Self { Self::new(self.x + r.x, self.y + r.y) } }
impl std::ops::AddAssign for Vec2 { fn add_assign(&mut self, r: Self) { self.x += r.x; self.y += r.y; } }
impl std::ops::Sub    for Vec2 { type Output = Self; fn sub(self, r: Self) -> Self { Self::new(self.x - r.x, self.y - r.y) } }
impl std::ops::SubAssign for Vec2 { fn sub_assign(&mut self, r: Self) { self.x -= r.x; self.y -= r.y; } }
impl std::ops::Mul<f32> for Vec2 { type Output = Self; fn mul(self, r: f32) -> Self { Self::new(self.x * r, self.y * r) } }
impl std::ops::MulAssign<f32> for Vec2 { fn mul_assign(&mut self, r: f32) { self.x *= r; self.y *= r; } }
impl std::ops::Div<f32>  for Vec2 { type Output = Self; fn div(self, r: f32)  -> Self { Self::new(self.x / r,   self.y / r)   } }
impl std::ops::Div<Vec2> for Vec2 { type Output = Self; fn div(self, r: Vec2) -> Self { Self::new(self.x / r.x, self.y / r.y) } }
impl std::ops::Neg      for Vec2 { type Output = Self; fn neg(self) -> Self { Self::new(-self.x, -self.y) } }

impl From<(f32, f32)>  for Vec2 { fn from((x, y): (f32, f32)) -> Self { Self::new(x, y) } }
impl From<[f32; 2]>    for Vec2 { fn from([x, y]: [f32; 2])   -> Self { Self::new(x, y) } }
impl From<Vec2> for [f32; 2]   { fn from(v: Vec2) -> Self { [v.x, v.y] } }

// ─── Color ───────────────────────────────────────────────────────────────────

/// RGBA colour (linear, 0.0 – 1.0).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[repr(C)]
pub struct Color {
    pub r: f32, pub g: f32, pub b: f32, pub a: f32,
}

impl Color {
    pub const WHITE:       Self = Self { r:1., g:1., b:1., a:1. };
    pub const BLACK:       Self = Self { r:0., g:0., b:0., a:1. };
    pub const RED:         Self = Self { r:1., g:0., b:0., a:1. };
    pub const GREEN:       Self = Self { r:0., g:1., b:0., a:1. };
    pub const BLUE:        Self = Self { r:0., g:0., b:1., a:1. };
    pub const YELLOW:      Self = Self { r:1., g:1., b:0., a:1. };
    pub const CYAN:        Self = Self { r:0., g:1., b:1., a:1. };
    pub const MAGENTA:     Self = Self { r:1., g:0., b:1., a:1. };
    pub const TRANSPARENT: Self = Self { r:0., g:0., b:0., a:0. };
    pub const DARK_GRAY:   Self = Self { r:0.15, g:0.15, b:0.15, a:1.0 };
    pub const GRAY:        Self = Self { r:0.5,  g:0.5,  b:0.5,  a:1.0 };
    pub const LIGHT_GRAY:  Self = Self { r:0.8,  g:0.8,  b:0.8,  a:1.0 };

    #[inline] pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }
    #[inline] pub fn rgb(r: f32, g: f32, b: f32) -> Self { Self { r, g, b, a: 1.0 } }
    #[inline] pub fn with_alpha(self, a: f32) -> Self { Self { a, ..self } }

    /// Pack to 0xAABBGGRR (little-endian RGBA).
    #[inline]
    pub fn to_rgba_u32(self) -> u32 {
        let r = (self.r.clamp(0., 1.) * 255.) as u32;
        let g = (self.g.clamp(0., 1.) * 255.) as u32;
        let b = (self.b.clamp(0., 1.) * 255.) as u32;
        let a = (self.a.clamp(0., 1.) * 255.) as u32;
        (a << 24) | (b << 16) | (g << 8) | r
    }

    #[inline]
    pub fn from_rgba_u32(c: u32) -> Self {
        Self {
            r: (c & 0xFF) as f32 / 255.,
            g: ((c >> 8)  & 0xFF) as f32 / 255.,
            b: ((c >> 16) & 0xFF) as f32 / 255.,
            a: ((c >> 24) & 0xFF) as f32 / 255.,
        }
    }

    /// Construct from 0xRRGGBB hex literal.
    #[inline]
    pub fn from_hex(hex: u32) -> Self {
        Self {
            r: ((hex >> 16) & 0xFF) as f32 / 255.,
            g: ((hex >> 8)  & 0xFF) as f32 / 255.,
            b: (hex & 0xFF) as f32 / 255.,
            a: 1.0,
        }
    }

    #[inline]
    pub fn lerp(self, o: Self, t: f32) -> Self {
        Self::new(
            self.r + (o.r - self.r) * t,
            self.g + (o.g - self.g) * t,
            self.b + (o.b - self.b) * t,
            self.a + (o.a - self.a) * t,
        )
    }

    /// Convert to HSV (h/s/v all in 0..1).
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let d   = max - min;
        let v   = max;
        let s   = if max > 0. { d / max } else { 0. };
        let h   = if d > 0. {
            let raw = if max == self.r      { 60.0 * ((self.g - self.b) / d)        }
                      else if max == self.g { 60.0 * ((self.b - self.r) / d + 2.0)  }
                      else                  { 60.0 * ((self.r - self.g) / d + 4.0)  };
            (if raw < 0. { raw + 360. } else { raw }) / 360.
        } else { 0. };
        (h, s, v)
    }

    /// Create from HSV (all parameters 0..1).
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        if s <= 0. { return Self::rgb(v, v, v); }
        let h6 = h * 6.;
        let i  = h6.floor() as i32;
        let f  = h6 - i as f32;
        let (p, q, t) = (v * (1. - s), v * (1. - s * f), v * (1. - s * (1. - f)));
        match i % 6 {
            0 => Self::rgb(v, t, p), 1 => Self::rgb(q, v, p),
            2 => Self::rgb(p, v, t), 3 => Self::rgb(p, q, v),
            4 => Self::rgb(t, p, v), _ => Self::rgb(v, p, q),
        }
    }
}

impl From<[f32; 4]> for Color { fn from([r,g,b,a]: [f32; 4]) -> Self { Self::new(r,g,b,a) } }
impl From<Color> for [f32; 4] { fn from(c: Color)             -> Self { [c.r, c.g, c.b, c.a] } }

// ─── Rect ────────────────────────────────────────────────────────────────────

/// Axis-aligned rectangle (min inclusive, max exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect { pub min: Vec2, pub max: Vec2 }

impl Rect {
    pub const ZERO: Self = Self { min: Vec2::ZERO, max: Vec2::ZERO };

    #[inline] pub fn new(min: Vec2, max: Vec2) -> Self { Self { min, max } }

    #[inline]
    pub fn from_min_size(min: Vec2, size: Vec2) -> Self {
        Self { min, max: min + size }
    }

    #[inline]
    pub fn from_center_size(center: Vec2, size: Vec2) -> Self {
        let h = size * 0.5;
        Self { min: center - h, max: center + h }
    }

    #[inline] pub fn width(self)  -> f32  { self.max.x - self.min.x }
    #[inline] pub fn height(self) -> f32  { self.max.y - self.min.y }
    #[inline] pub fn size(self)   -> Vec2 { Vec2::new(self.width(), self.height()) }
    #[inline] pub fn center(self) -> Vec2 { Vec2::new((self.min.x + self.max.x) * 0.5, (self.min.y + self.max.y) * 0.5) }
    #[inline] pub fn is_empty(self) -> bool { self.min.x >= self.max.x || self.min.y >= self.max.y }

    #[inline]
    pub fn contains(self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x < self.max.x && p.y >= self.min.y && p.y < self.max.y
    }

    #[inline]
    pub fn expand(self, amount: f32) -> Self {
        Self { min: Vec2::new(self.min.x - amount, self.min.y - amount),
               max: Vec2::new(self.max.x + amount, self.max.y + amount) }
    }

    #[inline] pub fn translate(self, d: Vec2) -> Self { Self { min: self.min + d, max: self.max + d } }
    #[inline] pub fn intersect(self, o: Self) -> Self { Self { min: self.min.max(o.min), max: self.max.min(o.max) } }
    #[inline] pub fn union(self, o: Self)     -> Self { Self { min: self.min.min(o.min), max: self.max.max(o.max) } }
    #[inline] pub fn overlaps(self, o: Self)  -> bool {
        o.min.x < self.max.x && o.max.x > self.min.x &&
        o.min.y < self.max.y && o.max.y > self.min.y
    }
}

// ─── WindowFlags ─────────────────────────────────────────────────────────────

bitflags::bitflags! {
    /// Controls window behaviour.
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct WindowFlags: u32 {
        const NO_TITLE_BAR       = 1 << 0;
        const NO_RESIZE          = 1 << 1;
        const NO_MOVE            = 1 << 2;
        const NO_SCROLLBAR       = 1 << 3;
        const NO_BACKGROUND      = 1 << 4;
        const NO_CLOSE_BUTTON    = 1 << 5;
        const ALWAYS_ON_TOP      = 1 << 6;
        const NO_DECORATION      = Self::NO_TITLE_BAR.bits() | Self::NO_SCROLLBAR.bits();
        const NO_INTERACTION     = Self::NO_MOVE.bits()   | Self::NO_RESIZE.bits();
    }
}
