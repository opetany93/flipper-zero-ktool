//! Physical units shared by the sensor and UI layers.

use core::fmt;

/// A voltage in millivolts, deliberately integral.
///
/// `newlib-nano` float formatting is not something to rely on inside a FAP, so
/// voltages travel as whole millivolts and only [`Display`](fmt::Display) splits
/// them into volts and hundredths.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millivolts(pub u32);

impl Millivolts {
    /// Takes a millivolt figure out of floating-point calibration.
    ///
    /// Truncates toward zero, matching the arithmetic the calibration constants
    /// were fitted against. The cast saturates, so a broken calibration surfaces
    /// as `0.00 V` rather than as a wrapped-around value.
    pub fn from_mv_f32(mv: f32) -> Self {
        Self(mv as u32)
    }

    pub const fn saturating_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }
}

impl fmt::Display for Millivolts {
    /// Renders whole volts and hundredths, e.g. `13.82`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{:02}", self.0 / 1000, (self.0 % 1000) / 10)
    }
}
