//! Keyboard and mouse input state.

use crate::Vec2;

// ─── MouseButton ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton { Left = 0, Right = 1, Middle = 2 }

// ─── Key ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Key {
    // Editing
    Backspace, Delete, Enter, Tab,
    Left, Right, Up, Down,
    Home, End, PageUp, PageDown,
    // Common
    Escape, Space,
    A, C, V, X, Y, Z, // cut/copy/paste/undo/redo shortcuts
    // Modifiers treated as keys too
    LeftShift, RightShift,
    LeftCtrl, RightCtrl,
    LeftAlt, RightAlt,
    // Sentinel
    COUNT,
}

// ─── Modifiers ───────────────────────────────────────────────────────────────

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Modifiers: u8 {
        const CTRL  = 1 << 0;
        const SHIFT = 1 << 1;
        const ALT   = 1 << 2;
        const SUPER = 1 << 3;
    }
}

// ─── InputState ──────────────────────────────────────────────────────────────

/// Snapshot of input at the start of each frame.
///
/// Backends fill this in; the UI reads from it.
#[derive(Debug, Clone, Default)]
pub struct InputState {
    // Mouse
    pub mouse_pos:       Vec2,
    pub mouse_delta:     Vec2,
    pub mouse_wheel:     f32,
    pub mouse_down:      [bool; 3],
    pub mouse_clicked:   [bool; 3],  // rose this frame
    pub mouse_released:  [bool; 3],  // fell this frame
    pub mouse_double_clicked: [bool; 3],

    // Keyboard / text
    pub keys_down:  [bool; Key::COUNT as usize],
    pub keys_pressed: [bool; Key::COUNT as usize],  // pressed this frame
    pub modifiers:  Modifiers,
    pub text_input: String, // UTF-8 characters typed this frame

    // Display
    pub display_size:   Vec2,
    pub delta_time:     f32,
    pub frame_count:    u64,
}

impl InputState {
    /// Call once per frame to roll pressed/clicked/released into the next frame.
    pub fn new_frame(&mut self) {
        self.mouse_clicked   = [false; 3];
        self.mouse_released  = [false; 3];
        self.mouse_double_clicked = [false; 3];
        self.keys_pressed    = [false; Key::COUNT as usize];
        self.mouse_delta     = Vec2::ZERO;
        self.mouse_wheel     = 0.0;
        self.text_input.clear();
        self.frame_count    += 1;
    }

    // ── builder helpers (call before new_frame for the coming frame) ──────────

    pub fn set_mouse_pos(&mut self, pos: Vec2) {
        self.mouse_delta = pos - self.mouse_pos;
        self.mouse_pos   = pos;
    }

    pub fn set_mouse_button(&mut self, btn: MouseButton, down: bool) {
        let i = btn as usize;
        if down && !self.mouse_down[i] { self.mouse_clicked[i]  = true; }
        if !down && self.mouse_down[i] { self.mouse_released[i] = true; }
        self.mouse_down[i] = down;
    }

    pub fn add_mouse_wheel(&mut self, y: f32) { self.mouse_wheel += y; }

    pub fn set_key(&mut self, key: Key, down: bool) {
        let i = key as usize;
        if down && !self.keys_down[i] { self.keys_pressed[i] = true; }
        self.keys_down[i] = down;
    }

    pub fn add_text(&mut self, ch: char) { self.text_input.push(ch); }

    // ── query helpers ─────────────────────────────────────────────────────────

    #[inline] pub fn mouse_down(&self, btn: MouseButton)     -> bool { self.mouse_down[btn as usize] }
    #[inline] pub fn mouse_clicked(&self, btn: MouseButton)  -> bool { self.mouse_clicked[btn as usize] }
    #[inline] pub fn mouse_released(&self, btn: MouseButton) -> bool { self.mouse_released[btn as usize] }

    #[inline] pub fn key_down(&self, k: Key)     -> bool { self.keys_down[k as usize] }
    #[inline] pub fn key_pressed(&self, k: Key)  -> bool { self.keys_pressed[k as usize] }

    #[inline] pub fn ctrl(&self)  -> bool { self.modifiers.contains(Modifiers::CTRL)  }
    #[inline] pub fn shift(&self) -> bool { self.modifiers.contains(Modifiers::SHIFT) }
    #[inline] pub fn alt(&self)   -> bool { self.modifiers.contains(Modifiers::ALT)   }
}
