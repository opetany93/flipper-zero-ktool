//! KTool - K-line / K-bus diagnostics for the Flipper Zero.
//!
//! The crate is layered, outermost first:
//!
//! - [`app`] - the event loop, and the state the GUI thread reads
//! - [`ui`] - what a frame looks like; pure drawing, no state
//! - [`sensor`] - physical quantities: sampling and calibration
//! - [`hal`] - safe, owning wrappers over the Furi C API
//!
//! Dependencies only ever point downwards, and `unsafe` appears only in
//! [`hal`]. Everything above it deals in ordinary Rust types whose lifetimes
//! the compiler checks.

#![no_main]
#![no_std]

// Provides the panic handler.
extern crate flipperzero_rt;
// Provides the `#[global_allocator]` backed by the Furi heap.
extern crate flipperzero_alloc;

mod app;
mod event;
mod hal;
mod sensor;
mod text;
mod ui;
mod units;

use core::ffi::CStr;

use flipperzero::info;
use flipperzero_rt::{entry, manifest};

use crate::hal::adc::{Adc, AnalogInput, SamplingConfig};
use crate::sensor::calibration::VsSenseCalibration;
use crate::sensor::vs_divider::VsDividerSensor;

manifest!(
    name = "KTool",
    app_version = 1,
    stack_size = 2 * 1024,
    has_icon = true,
    icon = "ktool.icon",
);

entry!(main);

/// Composition root.
///
/// The one place that knows which concrete sensor the app runs on. Everything
/// below takes the [`SupplyVoltageSource`](sensor::SupplyVoltageSource)
/// abstraction instead, so swapping the hardware means editing these few lines
/// and nothing else.
fn main(_args: Option<&CStr>) -> i32 {
    let mut supply = VsDividerSensor::new(
        Adc::acquire(SamplingConfig::HIGH_IMPEDANCE_2V5),
        AnalogInput::ext_pc3(),
        VsSenseCalibration::MEASURED,
    );

    info!("started, ADC on PC3 (channel 4)");

    app::run(&mut supply);

    0
}
