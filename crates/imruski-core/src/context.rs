//! Per-frame context – owns all mutable GUI state.

use std::collections::HashMap;
use ahash::RandomState;

/// A HashMap with a fixed-seed ahash hasher — no TLS, no runtime RNG.
type FxMap<K, V> = HashMap<K, V, RandomState>;

fn new_fxmap<K, V>() -> FxMap<K, V> {
    HashMap::with_hasher(RandomState::with_seeds(0xdeadbeef, 0xcafebabe, 0x12345678, 0xabcdef01))
}

use crate::{

    draw_list::DrawList,
    id::Id,
    input::InputState,
    layout::Layout,
    style::Style,
    Vec2, Rect, WindowFlags,
};

// ─── Persistent window state ─────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct WindowState {
    pub pos:       Vec2,
    pub size:      Vec2,
    pub collapsed: bool,
    pub scroll:    Vec2,
    pub flags:     WindowFlags,
}

impl WindowState {
    pub fn new(pos: Vec2, size: Vec2, flags: WindowFlags) -> Self {
        Self { pos, size, collapsed: false, scroll: Vec2::ZERO, flags }
    }
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.size)
    }
}

// ─── Persistent per-widget storage ───────────────────────────────────────────

/// Small state blob stored for a widget between frames.
#[derive(Debug, Clone, Default)]
pub struct WidgetStorage {
    pub float:  [f32; 4],
    pub int:    [i32; 4],
    pub string: String,
    pub active: bool,
    pub open:   bool,
}

// ─── Active window stack entry ────────────────────────────────────────────────

#[derive(Debug)]
pub(crate) struct WindowFrame {
    pub id:     Id,
    pub layout: Layout,
    pub draw_start: usize, // index into draw_list.cmd_buf where this window started
}

// ─── Context ─────────────────────────────────────────────────────────────────

/// The central state container. Create one per render target.
///
/// ```rust
/// let mut ctx = Context::new();
/// ctx.input_mut().set_mouse_pos(Vec2::new(100.0, 200.0));
/// ctx.new_frame();
/// // … call ui methods …
/// let frame = ctx.end_frame();
/// renderer.render(frame);
/// ```
#[derive(Debug)]
pub struct Context {
    pub(crate) style:          Style,
    pub(crate) input:          InputState,
    pub(crate) draw_list:      DrawList,

    // Persistent state maps (keyed by widget/window ID)
    pub(crate) windows:        FxMap<Id, WindowState>,
    pub(crate) widget_storage: FxMap<Id, WidgetStorage>,

    // Per-frame window stack
    pub(crate) window_stack:   Vec<WindowFrame>,

    // Focus / interaction tracking
    pub(crate) hot_item:    Option<Id>,   // hovered
    pub(crate) active_item: Option<Id>,   // being pressed/dragged
    pub(crate) focus_item:  Option<Id>,   // keyboard focus

    // ID stack (pushed/popped by the user)
    pub(crate) id_stack: Vec<Id>,

    // Window draw-order (back → front)
    pub(crate) window_order: Vec<Id>,

    // Tooltip buffer (rendered on top at end of frame)
    pub(crate) tooltip: Option<String>,

    // Delta time passed from the backend
    pub(crate) delta_time: f32,
}

impl Default for Context {
    fn default() -> Self { Self::new() }
}

impl Context {
    pub fn new() -> Self {
        Self {
            style:          Style::dark(),
            input:          InputState::default(),
            draw_list:      DrawList::default(),
            windows:        new_fxmap(),
            widget_storage: new_fxmap(),
            window_stack:   Vec::new(),
            hot_item:       None,
            active_item:    None,
            focus_item:     None,
            id_stack:       Vec::new(),
            window_order:   Vec::new(),
            tooltip:        None,
            delta_time:     0.016,
        }
    }

    // ── Configuration ─────────────────────────────────────────────────────────

    pub fn style(&self) -> &Style { &self.style }
    pub fn style_mut(&mut self) -> &mut Style { &mut self.style }
    pub fn set_display_size(&mut self, sz: Vec2) { self.input.display_size = sz; }
    pub fn set_delta_time(&mut self, dt: f32)    { self.delta_time = dt; }
    pub fn input_mut(&mut self) -> &mut InputState { &mut self.input }
    pub fn input(&self)         -> &InputState     { &self.input }

    // ── Frame lifecycle ───────────────────────────────────────────────────────

    /// Begin a new frame. Call this before any widget methods.
    pub fn new_frame(&mut self) {
        self.input.new_frame();
        self.draw_list.clear();
        self.window_stack.clear();
        self.tooltip = None;

        // Release active item if mouse was released
        use crate::input::MouseButton;
        if self.input.mouse_released(MouseButton::Left) {
            self.active_item = None;
        }
    }

    /// End the frame and return a render frame.
    pub fn end_frame(&self) -> crate::renderer::RenderFrame<'_> {
        crate::renderer::RenderFrame {
            draw_list:    &self.draw_list,
            display_size:  self.input.display_size,
            scale_factor:  1.0,
        }
    }

    // ── ID helpers ────────────────────────────────────────────────────────────

    pub fn push_id(&mut self, id: impl Into<u64>) { self.id_stack.push(Id(id.into())); }
    pub fn push_id_str(&mut self, s: &str) { self.id_stack.push(Id::from_str(s)); }
    pub fn pop_id(&mut self) { self.id_stack.pop(); }

    pub(crate) fn make_id(&self, label_id: &str) -> Id {
        let base = Id::from_str(label_id);
        // Combine with all IDs in the stack
        self.id_stack.iter().fold(base, |acc, &id| acc.combine(id))
    }

    // ── Interaction helpers ───────────────────────────────────────────────────

    pub(crate) fn is_hot(&self, id: Id)    -> bool { self.hot_item    == Some(id) }
    pub(crate) fn is_active(&self, id: Id) -> bool { self.active_item == Some(id) }
    pub(crate) fn is_focused(&self, id: Id)-> bool { self.focus_item  == Some(id) }

    /// Test a rect against mouse and update hot/active.
    /// Returns `(hovered, held, clicked)`.
    pub(crate) fn button_behavior(
        &mut self,
        id:   Id,
        rect: Rect,
    ) -> (bool, bool, bool) {
        use crate::input::MouseButton;
        let hovered = rect.contains(self.input.mouse_pos);
        if hovered { self.hot_item = Some(id); }

        let active  = self.active_item == Some(id);
        let clicked;

        if hovered && self.input.mouse_clicked(MouseButton::Left) {
            self.active_item = Some(id);
            self.focus_item  = Some(id);
            clicked = false;
        } else if active && self.input.mouse_released(MouseButton::Left) {
            clicked = hovered;
            // active_item cleared in new_frame
        } else {
            clicked = false;
        }

        (hovered, active || (hovered && self.input.mouse_down(MouseButton::Left) && active), clicked)
    }

    // ── Widget storage ────────────────────────────────────────────────────────

    pub(crate) fn get_storage(&self, id: Id) -> Option<&WidgetStorage> {
        self.widget_storage.get(&id)
    }
    pub(crate) fn get_storage_mut(&mut self, id: Id) -> &mut WidgetStorage {
        self.widget_storage.entry(id).or_default()
    }

    // ── Current window helpers ────────────────────────────────────────────────

    pub(crate) fn current_window(&self) -> Option<&WindowFrame> {
        self.window_stack.last()
    }

    pub(crate) fn current_window_mut(&mut self) -> Option<&mut WindowFrame> {
        self.window_stack.last_mut()
    }

    pub(crate) fn current_layout_mut(&mut self) -> Option<&mut Layout> {
        self.window_stack.last_mut().map(|w| &mut w.layout)
    }

    // ── Draw list passthrough ─────────────────────────────────────────────────

    pub(crate) fn draw_list_mut(&mut self) -> &mut DrawList { &mut self.draw_list }
}
