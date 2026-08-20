//! The ADC, and the header pin KTool samples with it.

use flipperzero_sys as sys;

/// A header pin wired to an ADC channel, already switched to analog mode.
#[derive(Clone, Copy, Debug)]
pub struct AnalogInput {
    channel: sys::FuriHalAdcChannel,
}

impl AnalogInput {
    /// Header pin 7 - PC3 - `ADC1_IN4`, where the VS-sense divider is tapped.
    ///
    /// Switches the pin to analog mode, rather than leaving that as a separate
    /// step a caller can forget.
    pub fn ext_pc3() -> Self {
        // SAFETY: `gpio_ext_pc3` is a static pin descriptor exported by the
        // firmware, and `furi_hal_gpio_init` only reads through the pointer.
        unsafe {
            sys::furi_hal_gpio_init(
                &raw const sys::gpio_ext_pc3,
                sys::GpioModeAnalog,
                sys::GpioPullNo,
                sys::GpioSpeedLow,
            );
        }

        Self {
            channel: sys::FuriHalAdcChannel4,
        }
    }
}

/// How the ADC is clocked, oversampled and held.
///
/// Whole named profiles rather than four independent knobs: the calibration is
/// fitted against one specific combination, and mixing settings that were never
/// measured together is the mistake worth making impossible.
#[derive(Clone, Copy, Debug)]
pub struct SamplingConfig {
    scale: sys::FuriHalAdcScale,
    clock: sys::FuriHalAdcClock,
    oversample: sys::FuriHalAdcOversample,
    sampling_time: sys::FuriHalAdcSamplingTime,
}

impl SamplingConfig {
    /// 2.5 V full scale, 64x oversampling, 247.5-cycle sample window.
    ///
    /// The long window is not optional for a high-impedance source. The VS tap
    /// looks like ~9.4k to the ADC input, and a short window leaves the
    /// sample-and-hold capacitor undercharged: the reading comes out low but
    /// perfectly stable, and therefore easy to mistake for a correct one.
    pub const HIGH_IMPEDANCE_2V5: Self = Self {
        scale: sys::FuriHalAdcScale2500,
        clock: sys::FuriHalAdcClockSync64,
        oversample: sys::FuriHalAdcOversample64,
        sampling_time: sys::FuriHalAdcSamplingtime247_5,
    };
}

/// An acquired ADC. Releases the peripheral, and its power and clock domains,
/// on drop.
pub struct Adc {
    handle: *mut sys::FuriHalAdcHandle,
}

impl Adc {
    /// Acquires the ADC and brings it up with `config`.
    pub fn acquire(config: SamplingConfig) -> Self {
        // SAFETY: `furi_hal_adc_acquire` blocks until it can hand out a valid
        // handle, configured here before any caller can read through it.
        let handle = unsafe {
            let handle = sys::furi_hal_adc_acquire();
            sys::furi_hal_adc_configure_ex(
                handle,
                config.scale,
                config.clock,
                config.oversample,
                config.sampling_time,
            );
            handle
        };

        Self { handle }
    }

    /// Runs one conversion on `input` and returns the voltage at the pin, in
    /// millivolts, after the handle's own factory calibration.
    ///
    /// Blocking and slow by GUI standards: application thread only, never a
    /// draw or input callback.
    pub fn read(&mut self, input: AnalogInput) -> f32 {
        // SAFETY: `self.handle` is valid for as long as `self`, and `&mut self`
        // rules out a second conversion in flight on it.
        unsafe {
            let raw = sys::furi_hal_adc_read(self.handle, input.channel);

            sys::furi_hal_adc_convert_to_voltage(self.handle, raw)
        }
    }
}

impl Drop for Adc {
    fn drop(&mut self) {
        // SAFETY: the handle came from `furi_hal_adc_acquire` and, since `Adc`
        // is neither `Clone` nor `Copy`, is released exactly once.
        unsafe { sys::furi_hal_adc_release(self.handle) };
    }
}
