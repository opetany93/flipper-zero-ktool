//! The fitted correction from ADC pin voltage to vehicle voltages.
//!
//! Pure arithmetic, no hardware.

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
    /// Fitted 2026-08-19 on the assembled board, seven points from 4 to 16 V
    /// off a bench supply.
    ///
    /// The divider is R3 150k over R4 10k, so the nominal ratio is
    /// `(150k + 10k) / 10k = 16.0`; the fitted 16.142 is +0.9%, inside the 1%
    /// resistor tolerance. Against the raw code the fit is
    /// `VS = 9.8585 * raw + 34.1 mV`, residuals between -5.1 and +3.5 mV. That
    /// is the floor rather than a good result: one ADC count is 9.86 mV at the
    /// VS node, so quantisation alone accounts for +/-4.9 mV. Against a fresh
    /// multimeter reading expect about +/-10 mV, since the residual and the
    /// quantisation of that particular sample both count.
    ///
    /// D1 is a 1N5819 Schottky and its drop is **not** constant - it runs from
    /// 150 mV at 4 V to 199 mV at 16 V, following current. 193 mV is the middle
    /// of the 10-16 V operating range (186-199 mV), so B+ holds +/-7 mV there
    /// and reads low below it. The earlier 117 mV was measured before the
    /// transceivers were soldered on, when the board drew less.
    ///
    /// The gain was then checked by temporarily displaying VS in whole
    /// millivolts: `pin_mv / raw` came out at 0.6102, implying 16.153 against
    /// the fitted 16.142. That is 11 mV at 16 V, the same size as the
    /// uncertainty of the check itself, so the seven-point fit was kept over
    /// the two-point one. Nothing is left to trim without a finer ADC step, and
    /// that is a hardware question: 16 V reaches only 990 mV of the 2500 mV
    /// scale, but the 16:1 ratio is set by load-dump survival, not resolution -
    /// D2 clamps at ~41 V, which lands at 2.56 V on a 3.3 V pin. A steeper
    /// divider would need a clamp diode at the pin to go with it.
    ///
    /// These numbers are tied to
    /// [`SamplingConfig::HIGH_IMPEDANCE_2V5`](crate::hal::adc::SamplingConfig::HIGH_IMPEDANCE_2V5).
    /// Change the scale, the oversampling or the sample window and the fit has
    /// to be redone.
    pub const MEASURED: Self = Self {
        gain: 16.142,
        offset_mv: 34.1,
        d1_forward_drop: Millivolts(193),
    };

    /// Lowest voltage the fit was made against.
    ///
    /// Below it the affine model has no data behind it and degenerates into the
    /// bare offset, which reads as a plausible small voltage rather than as the
    /// absence of one. The same floor is used for B+, although the D1 constant
    /// is only really trustworthy from 10 V up; hiding B+ between 4 and 10 V
    /// would cost more than the ~14 mV it is off by there.
    pub const VALID_FROM: Millivolts = Millivolts(4_000);

    /// Voltage at the VS node, from the voltage measured at the divider tap.
    ///
    /// `None` below [`VALID_FROM`](Self::VALID_FROM).
    pub fn vs(&self, pin_mv: f32) -> Option<Millivolts> {
        let vs = Millivolts::from_mv_f32(pin_mv * self.gain + self.offset_mv);

        (vs >= Self::VALID_FROM).then_some(vs)
    }

    /// Voltage upstream of D1, reconstructed from the VS node.
    pub fn b_plus(&self, vs: Millivolts) -> Millivolts {
        vs.saturating_add(self.d1_forward_drop)
    }
}
