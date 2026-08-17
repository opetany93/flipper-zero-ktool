//! Turning what the Kextension board exposes into numbers a screen can show.

pub mod calibration;
pub mod vs_divider;

use crate::units::Millivolts;

/// One reading of the vehicle supply.
///
/// Both voltages are worked out here rather than in the UI: where D1 sits and
/// what it costs is a property of the board, not of a screen layout.
#[derive(Clone, Copy, Default, Debug)]
pub struct SupplyReading {
    /// Raw ADC code. On screen because it is the first thing worth looking at
    /// when a voltage looks wrong.
    pub adc_raw: u16,
    /// Voltage at the VS node, behind D1.
    pub vs: Millivolts,
    /// Voltage upstream of D1: what the car is actually supplying.
    pub b_plus: Millivolts,
}

/// A source of supply readings.
///
/// The event loop depends on this rather than on
/// [`VsDividerSensor`](vs_divider::VsDividerSensor), so it never has to know
/// that an ADC exists - and so it keeps working unchanged the day the same
/// figure starts arriving over KWP2000 instead.
pub trait SupplyVoltageSource {
    /// Takes one reading.
    ///
    /// May block; called from the application thread only.
    fn read(&mut self) -> SupplyReading;
}
