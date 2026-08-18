//! The fitted correction from ADC pin voltage to vehicle voltages.
//!
//! Pure arithmetic, no hardware. This is the piece that gets re-measured when
//! the divider, the diode or the ADC configuration changes.

use crate::units::Millivolts;

/// Correction constants for the VS-sense path.
#[derive(Clone, Copy, Debug)]
pub struct VsSenseCalibration {
    /// Divider ratio, including the fitted gain error.
    pub gain: f32,
    /// Fixed offset of the measuring path, in millivolts.
    pub offset_mv: f32,
    /// Forward drop across D1 at this load.
    pub d1_forward_drop: Millivolts,
}

impl VsSenseCalibration {
    /// Fitted 2026-07-29 against a multimeter at 10 / 14 / 16 V.
    ///
    /// The divider is R3 150k over R4 10k, so the nominal ratio is
    /// `(150k + 10k) / 10k = 16.0`. The fit came out as
    /// `VS_true = 1.00531 * VS_read + 0.0779 V`: a small gain error from
    /// resistor tolerance, plus a fixed offset from ADC offset and input
    /// leakage across the ~9.4k source impedance. Residuals were
    /// +4.3 / -6.9 / +4.6 mV.
    ///
    /// D1 is a 1N5819 Schottky. Its measured drop is ~117 mV at this load
    /// (111 / 115 / 125 mV over 10-16 V), far below the 300 mV datasheet
    /// figure - that one applies at ~1 A, not at the ~15 mA this circuit draws.
    ///
    /// These numbers are tied to
    /// [`SamplingConfig::HIGH_IMPEDANCE_2V5`](crate::hal::adc::SamplingConfig::HIGH_IMPEDANCE_2V5).
    /// Change the scale, the oversampling or the sample window and the fit has
    /// to be redone.
    pub const MEASURED: Self = Self {
        gain: 16.085, // 16.0 nominal x 1.00531 measured
        offset_mv: 78.0,
        d1_forward_drop: Millivolts(117),
    };

    /// Voltage at the VS node, from the voltage measured at the divider tap.
    pub fn vs(&self, pin_mv: f32) -> Millivolts {
        Millivolts::from_mv_f32(pin_mv * self.gain + self.offset_mv)
    }

    /// Voltage upstream of D1, reconstructed from the VS node.
    pub fn b_plus(&self, vs: Millivolts) -> Millivolts {
        vs.saturating_add(self.d1_forward_drop)
    }
}
