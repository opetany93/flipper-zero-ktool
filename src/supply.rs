//! Measuring the vehicle supply voltage.

pub mod calibration;
pub mod divider;

use crate::units::Millivolts;

/// One reading of the vehicle supply.
///
/// Both voltages are computed here, not in the UI: where D1 sits and what it
/// costs is a property of the board.
#[derive(Clone, Copy, Default, Debug)]
pub struct Reading {
    /// Raw ADC code, on screen because it is the first thing to check when a
    /// voltage looks wrong.
    pub adc_raw: u16,
    /// Voltage at the VS node, behind D1.
    pub vs: Millivolts,
    /// Voltage upstream of D1: what the car is actually supplying.
    pub b_plus: Millivolts,
}

/// A source of supply readings.
///
/// The event loop depends on this rather than on [`VsDivider`](divider::VsDivider),
/// so the same figure can later arrive over KWP2000 without touching the loop.
pub trait VoltageSource {
    /// May block; called from the application thread only.
    fn read(&mut self) -> Reading;
}
