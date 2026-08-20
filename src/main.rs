//! KTool - K-line / K-bus diagnostics for the Flipper Zero.
//!
//! Layered, outermost first:
//!
//! - [`app`] - the event loop, and the state the GUI thread reads
//! - [`ui`] - what a frame looks like; pure drawing, no state
//! - [`supply`] - physical quantities: sampling and calibration
//! - [`hal`] - safe, owning wrappers over the Furi C API
//!
//! Dependencies point downwards only, and `unsafe` appears only in [`hal`].

#![no_main]
#![no_std]

// Provides the panic handler.
extern crate flipperzero_rt;
// Provides the `#[global_allocator]` backed by the Furi heap.
extern crate flipperzero_alloc;

mod app;
mod event;
mod hal;
mod supply;
mod text;
mod ui;
mod units;

use core::ffi::CStr;

use flipperzero::info;
use flipperzero_rt::{entry, manifest};

use crate::hal::adc::{Adc, AnalogInput, SamplingConfig};
use crate::supply::calibration::VsSenseCalibration;
use crate::supply::divider::VsDivider;

manifest!(
    name = "KTool",
    app_version = 1,
    stack_size = 2 * 1024,
    has_icon = true,
    icon = "ktool.icon",
);

entry!(main);

/// Composition root: the one place that knows which concrete source the app
/// runs on. Everything below takes the [`VoltageSource`](supply::VoltageSource)
/// abstraction instead.
fn main(_args: Option<&CStr>) -> i32 {
    let mut supply = VsDivider::new(
        Adc::acquire(SamplingConfig::HIGH_IMPEDANCE_2V5),
        AnalogInput::ext_pc3(),
        VsSenseCalibration::MEASURED,
    );

    info!("started, ADC on PC3 (channel 4)");

    app::run(&mut supply);

    0
}
