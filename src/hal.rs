//! Safe wrappers over the slices of the Furi C API that KTool uses.
//!
//! The only place in the crate that mentions `flipperzero_sys`, so every
//! `unsafe` block and its justification lives here. Each wrapper owns its C
//! resource and releases it on drop.

pub mod adc;
pub mod canvas;
pub mod input;
pub mod timer;
pub mod view_port;
