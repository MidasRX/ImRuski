//! Visual style / theming.

use crate::Color;

// ─── StyleColor indices ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum StyleColor {
    WindowBg = 0,
    WindowBorder,
    TitleBar,
    TitleBarText,
    TitleBarActive,
    ChildBg,
    PopupBg,
    Border,
    FrameBg,
    FrameBgHovered,
    FrameBgActive,
    Text,
    TextDisabled,
    Button,
    ButtonHovered,
    ButtonActive,
    CheckMark,
    SliderGrab,
    SliderGrabActive,
    Header,
    HeaderHovered,
    HeaderActive,
    Separator,
    Tab,
    TabHovered,
    TabActive,
    ScrollbarBg,
    ScrollbarGrab,
    ScrollbarGrabHovered,
    ScrollbarGrabActive,
    ResizeGrip,
    ResizeGripHovered,
    ResizeGripActive,
    PlotLines,
    PlotHistogram,
    ProgressBar,
    // Sentinel – always last
    COUNT,
}

// ─── StyleVar ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub enum StyleVar {
    WindowPadding(f32, f32),
    ItemSpacing(f32, f32),
    FramePadding(f32, f32),
    WindowRounding(f32),
    FrameRounding(f32),
    ScrollbarSize(f32),
    GrabMinSize(f32),
    Alpha(f32),
}

// ─── Style ───────────────────────────────────────────────────────────────────

/// Full visual style specification.
#[derive(Debug, Clone)]
pub struct Style {
    pub colors: [Color; StyleColor::COUNT as usize],

    // Layout
    pub window_padding:   (f32, f32),
    pub window_rounding:  f32,
    pub window_min_size:  (f32, f32),
    pub window_title_height: f32,
    pub item_spacing:     (f32, f32),
    pub frame_padding:    (f32, f32),
    pub frame_rounding:   f32,
    pub indent_spacing:   f32,
    pub scrollbar_size:   f32,
    pub grab_min_size:    f32,
    pub alpha:            f32,
    pub font_size:        f32,
}

impl Default for Style {
    fn default() -> Self { Self::dark() }
}

impl Style {
    /// Classic Dear-ImGui dark theme.
    pub fn dark() -> Self {
        use StyleColor as SC;
        let mut colors = [Color::TRANSPARENT; SC::COUNT as usize];
        colors[SC::WindowBg      as usize] = Color::from_hex(0x0F0F0F).with_alpha(0.94);
        colors[SC::WindowBorder  as usize] = Color::from_hex(0x404040);
        colors[SC::TitleBar      as usize] = Color::from_hex(0x1a1a2e);
        colors[SC::TitleBarText  as usize] = Color::WHITE;
        colors[SC::TitleBarActive as usize] = Color::from_hex(0x16213e);
        colors[SC::ChildBg       as usize] = Color::TRANSPARENT;
        colors[SC::PopupBg       as usize] = Color::from_hex(0x0d0d0d).with_alpha(0.95);
        colors[SC::Border        as usize] = Color::from_hex(0x404040);
        colors[SC::FrameBg       as usize] = Color::from_hex(0x292929);
        colors[SC::FrameBgHovered as usize] = Color::from_hex(0x3d3d3d);
        colors[SC::FrameBgActive  as usize] = Color::from_hex(0x1e6bb5);
        colors[SC::Text          as usize] = Color::from_hex(0xe8e8e8);
        colors[SC::TextDisabled  as usize] = Color::from_hex(0x808080);
        colors[SC::Button        as usize] = Color::from_hex(0x1e6bb5).with_alpha(0.9);
        colors[SC::ButtonHovered  as usize] = Color::from_hex(0x3d8ed5);
        colors[SC::ButtonActive   as usize] = Color::from_hex(0x1753a0);
        colors[SC::CheckMark     as usize] = Color::from_hex(0x4db5ff);
        colors[SC::SliderGrab    as usize] = Color::from_hex(0x4db5ff);
        colors[SC::SliderGrabActive as usize] = Color::from_hex(0x80caff);
        colors[SC::Header        as usize] = Color::from_hex(0x1e6bb5).with_alpha(0.7);
        colors[SC::HeaderHovered  as usize] = Color::from_hex(0x3d8ed5).with_alpha(0.8);
        colors[SC::HeaderActive   as usize] = Color::from_hex(0x1e6bb5);
        colors[SC::Separator     as usize] = Color::from_hex(0x404040);
        colors[SC::Tab           as usize] = Color::from_hex(0x1a1a2e);
        colors[SC::TabHovered    as usize] = Color::from_hex(0x3d8ed5);
        colors[SC::TabActive     as usize] = Color::from_hex(0x1e6bb5);
        colors[SC::ScrollbarBg   as usize] = Color::from_hex(0x080808).with_alpha(0.53);
        colors[SC::ScrollbarGrab  as usize] = Color::from_hex(0x1f1f1f);
        colors[SC::ScrollbarGrabHovered as usize] = Color::from_hex(0x3a3a3a);
        colors[SC::ScrollbarGrabActive  as usize] = Color::from_hex(0x565656);
        colors[SC::ResizeGrip    as usize] = Color::from_hex(0x1e6bb5).with_alpha(0.4);
        colors[SC::ResizeGripHovered as usize] = Color::from_hex(0x4db5ff).with_alpha(0.6);
        colors[SC::ResizeGripActive  as usize] = Color::from_hex(0x4db5ff).with_alpha(0.9);
        colors[SC::PlotLines     as usize] = Color::from_hex(0x9a9a9a);
        colors[SC::PlotHistogram as usize] = Color::from_hex(0xe6b400);
        colors[SC::ProgressBar   as usize] = Color::from_hex(0x1e6bb5);
        Self {
            colors,
            window_padding:      (8.0, 8.0),
            window_rounding:     4.0,
            window_min_size:     (32.0, 32.0),
            window_title_height: 22.0,
            item_spacing:        (8.0, 4.0),
            frame_padding:       (4.0, 3.0),
            frame_rounding:      3.0,
            indent_spacing:      21.0,
            scrollbar_size:      14.0,
            grab_min_size:       10.0,
            alpha:               1.0,
            font_size:           13.0,
        }
    }

    /// Light theme variant.
    pub fn light() -> Self {
        let mut s = Self::dark();
        use StyleColor as SC;
        s.colors[SC::WindowBg   as usize] = Color::from_hex(0xf0f0f0);
        s.colors[SC::TitleBar   as usize] = Color::from_hex(0x4293d1);
        s.colors[SC::Text       as usize] = Color::BLACK;
        s.colors[SC::FrameBg    as usize] = Color::from_hex(0xdedede);
        s.colors[SC::Button     as usize] = Color::from_hex(0x4293d1);
        s
    }

    /// Convenience accessor.
    #[inline] pub fn color(&self, c: StyleColor) -> Color { self.colors[c as usize] }

    /// Push a temporary style override, returning the old value.
    pub fn push_var(&mut self, var: StyleVar) -> StyleVarRestore {
        match var {
            StyleVar::WindowPadding(x, y) => {
                let old = self.window_padding;
                self.window_padding = (x, y);
                StyleVarRestore::Padding2(StyleVarKind::WindowPadding, old)
            }
            StyleVar::ItemSpacing(x, y) => {
                let old = self.item_spacing;
                self.item_spacing = (x, y);
                StyleVarRestore::Padding2(StyleVarKind::ItemSpacing, old)
            }
            StyleVar::FramePadding(x, y) => {
                let old = self.frame_padding;
                self.frame_padding = (x, y);
                StyleVarRestore::Padding2(StyleVarKind::FramePadding, old)
            }
            StyleVar::WindowRounding(v) => {
                let old = self.window_rounding;
                self.window_rounding = v;
                StyleVarRestore::Float(StyleVarKind::WindowRounding, old)
            }
            StyleVar::FrameRounding(v) => {
                let old = self.frame_rounding;
                self.frame_rounding = v;
                StyleVarRestore::Float(StyleVarKind::FrameRounding, old)
            }
            StyleVar::ScrollbarSize(v) => {
                let old = self.scrollbar_size;
                self.scrollbar_size = v;
                StyleVarRestore::Float(StyleVarKind::ScrollbarSize, old)
            }
            StyleVar::GrabMinSize(v) => {
                let old = self.grab_min_size;
                self.grab_min_size = v;
                StyleVarRestore::Float(StyleVarKind::GrabMinSize, old)
            }
            StyleVar::Alpha(v) => {
                let old = self.alpha;
                self.alpha = v;
                StyleVarRestore::Float(StyleVarKind::Alpha, old)
            }
        }
    }

    pub fn pop_var(&mut self, r: StyleVarRestore) {
        match r {
            StyleVarRestore::Float(k, v) => match k {
                StyleVarKind::WindowRounding => self.window_rounding = v,
                StyleVarKind::FrameRounding  => self.frame_rounding  = v,
                StyleVarKind::ScrollbarSize  => self.scrollbar_size  = v,
                StyleVarKind::GrabMinSize    => self.grab_min_size   = v,
                StyleVarKind::Alpha          => self.alpha           = v,
                _ => {}
            },
            StyleVarRestore::Padding2(k, (x, y)) => match k {
                StyleVarKind::WindowPadding => self.window_padding = (x, y),
                StyleVarKind::ItemSpacing   => self.item_spacing   = (x, y),
                StyleVarKind::FramePadding  => self.frame_padding  = (x, y),
                _ => {}
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StyleVarKind {
    WindowPadding, ItemSpacing, FramePadding,
    WindowRounding, FrameRounding, ScrollbarSize, GrabMinSize, Alpha,
}

#[derive(Debug, Clone, Copy)]
pub enum StyleVarRestore {
    Float(StyleVarKind, f32),
    Padding2(StyleVarKind, (f32, f32)),
}
