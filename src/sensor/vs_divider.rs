//! Supply sensing through the VS divider on the Kextension board.

use crate::hal::adc::{Adc, AnalogInput};
use crate::sensor::calibration::VsSenseCalibration;
use crate::sensor::{SupplyReading, SupplyVoltageSource};

/// Reads the vehicle supply through the 150k / 10k divider tapped on PC3.
pub struct VsDividerSensor {
    adc: Adc,
    input: AnalogInput,
    calibration: VsSenseCalibration,
}

impl VsDividerSensor {
    /// Assembles a sensor from the three things it needs: a configured ADC, the
    /// pin the divider is tapped on, and the fit that turns volts at that pin
    /// into volts in the car.
    pub fn new(adc: Adc, input: AnalogInput, calibration: VsSenseCalibration) -> Self {
        Self {
            adc,
            input,
            calibration,
        }
    }
}

impl SupplyVoltageSource for VsDividerSensor {
    fn read(&mut self) -> SupplyReading {
        let sample = self.adc.read(self.input);
        let vs = self.calibration.vs(sample.pin_mv);

        SupplyReading {
            adc_raw: sample.raw,
            vs,
            b_plus: self.calibration.b_plus(vs),
        }
    }
}
