//! Plugin SDK — pure trait definitions for plugin authors.
//! No runtime dependencies. Plugins implement these traits.

pub mod hooks;
pub mod plugin;
pub mod registration;

pub use hooks::*;
pub use plugin::*;
pub use registration::*;
