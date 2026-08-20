//! Supply sensing through the VS divider on the Kextension board.

use crate::hal::adc::{Adc, AnalogInput};
use crate::supply::calibration::VsSenseCalibration;
use crate::supply::{Reading, VoltageSource};

/// Reads the vehicle supply through the 150k / 10k divider tapped on PC3.
pub struct VsDivider {
    adc: Adc,
    input: AnalogInput,
    calibration: VsSenseCalibration,
}

impl VsDivider {
    pub fn new(adc: Adc, input: AnalogInput, calibration: VsSenseCalibration) -> Self {
        Self {
            adc,
            input,
            calibration,
        }
    }
}

impl VoltageSource for VsDivider {
    fn read(&mut self) -> Reading {
        let vs = self.calibration.vs(self.adc.read(self.input));

        Reading {
            vs,
            b_plus: vs.map(|vs| self.calibration.b_plus(vs)),
        }
    }
}
