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
use crate::hal::serial::{self, Port, SerialPort};
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

    // K-line is on USART1 (header pins 13/14) at 10400 8N1, K-bus on LPUART1
    // (15/16) at 9600 8E1.
    //
    // A busy port means a Flipper setting rather than a fault - the log device,
    // or the Expansion Modules service - so the app carries on with the supply
    // reading and says so on screen instead of refusing to start.
    let kline_serial_port = SerialPort::open(Port::Usart, 10_400, serial::Framing::EIGHT_N1).ok();
    let kbus_serial_port = SerialPort::open(Port::Lpuart, 9_600, serial::Framing::EIGHT_E1).ok();

    app::run(&mut supply, kline_serial_port, kbus_serial_port);

    0
}
