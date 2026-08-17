//! Physical units shared by the sensor and UI layers.

use core::fmt;

/// A voltage in millivolts.
///
/// Deliberately integral. `newlib-nano` float formatting is not something to
/// rely on inside a FAP, so voltages travel as whole millivolts and the
/// volts-and-hundredths split happens in [`Display`](fmt::Display) - once, here,
/// instead of at every call site that wants to print one.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Millivolts(pub u32);

impl Millivolts {
    /// Takes a millivolt figure out of floating-point calibration.
    ///
    /// Truncates toward zero, matching the arithmetic the calibration constants
    /// were fitted against. Rust's float-to-integer casts saturate, so a
    /// negative or NaN result of a broken calibration surfaces as `0.00 V`
    /// rather than as a wrapped-around value.
    pub fn from_mv_f32(mv: f32) -> Self {
        Self(mv as u32)
    }

    /// Adds two voltages, clamping rather than wrapping at the top.
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
