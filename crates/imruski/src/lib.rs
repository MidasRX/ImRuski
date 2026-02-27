//! # ImRuski
//!
//! Immediate-mode GUI library for Rust.  Single umbrella crate that re-exports
//! the core engine and all optional backends.
//!
//! ## Feature flags
//!
//! | Flag          | Backend                     |
//! |---------------|-----------------------------|
//! | `sciter`      | Sciter (HTML engine window) |
//! | `ultralight`  | Ultralight GPU renderer     |
//! | `dx11`        | DirectX 11 game overlay     |
//! | `memory`      | toy-arms memory utilities   |
//! | `full`        | All of the above            |
//!
//! ## Minimal example
//!
//! ```rust
//! use imruski::prelude::*;
//!
//! let mut ctx = Context::new();
//! ctx.set_display_size(Vec2::new(1280.0, 720.0));
//! // … attach a backend renderer and call ctx.frame(…) each tick …
//! ```

// ─── Core re-exports ─────────────────────────────────────────────────────────

pub use imruski_core::*;
pub use imruski_core as core;

/// String obfuscation utilities (wraps `obfstr`).
/// Use `obfstr::obfstr!("sensitive string")` in your code.
pub use obfstr;

// ─── Backend modules ─────────────────────────────────────────────────────────

#[cfg(feature = "sciter")]
pub mod sciter {
    //! Sciter-based rendering backend.
    pub use imruski_sciter::*;
}

#[cfg(feature = "ultralight")]
pub mod ultralight {
    //! Ultralight GPU rendering backend.
    pub use imruski_ultralight::*;
}

#[cfg(all(feature = "dx11", target_os = "windows"))]
pub mod dx11 {
    //! DirectX 11 game-overlay backend.
    pub use imruski_dx11::*;
}

// ─── Memory utilities (toy-arms) ─────────────────────────────────────────────

#[cfg(all(feature = "memory", target_os = "windows"))]
pub mod memory {
    //! Windows process-memory utilities powered by `toy-arms`.
    //!
    //! # Example
    //! ```no_run
    //! use imruski::memory::Process;
    //!
    //! let proc = Process::from_process_name("game.exe").unwrap();
    //! let health: f32 = proc.read(0x1234_5678).unwrap();
    //! ```
    pub use toy_arms::*;
}

// ─── Prelude ─────────────────────────────────────────────────────────────────

pub mod prelude {
    pub use crate::{
        Color,
        Context,
        Rect,
        Vec2,
        WindowFlags,
        draw_list::TextureId,
        id::Id,
        input::{InputState, Key, Modifiers, MouseButton},
        renderer::Renderer,
        style::{Style, StyleColor, StyleVar},
        ui::Ui,
    };
}
