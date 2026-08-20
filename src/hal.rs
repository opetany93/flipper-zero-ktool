//! Safe wrappers over the slices of the Furi C API that KTool uses.
//!
//! This is the only place in the crate that mentions `flipperzero_sys`. Two
//! things follow from that:
//!
//! - Every `unsafe` block, and every argument about why it is sound, is in one
//!   directory instead of scattered through the application.
//! - Each wrapper owns its C resource and releases it on drop, which turns the
//!   shutdown ordering in [`crate::app`] from a comment that has to be obeyed
//!   into a property the compiler checks.

pub mod adc;
pub mod canvas;
pub mod input;
pub mod timer;
pub mod view_port;
