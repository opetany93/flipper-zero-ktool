#ifndef KTOOL_I_H
#define KTOOL_I_H

#include <furi.h>
#include <furi_hal_adc.h>
#include <gui/gui.h>

#define TAG "KTool"

/* How often the tick event is emitted (also the VS sampling period). */
#define KTOOL_TICK_PERIOD_MS 500

/* VS-sense divider: PC3 = ADC1_IN4, R3 150k (upper) / R4 10k (lower).
 *
 * Calibrated 2026-07-29 against a multimeter at 10 / 14 / 16 V.
 * Nominal gain is (R3 + R4) / R4 = (150k + 10k) / 10k = 16.0; the fitted
 * correction is VS_true = 1.00531 * VS_read + 0.0779 V, i.e. a small gain
 * error (resistor tolerance) plus a fixed offset (ADC offset / input leakage
 * across the ~9.4k source impedance). Residuals: +4.3 / -6.9 / +4.6 mV.
 *
 * WARNING: these constants are tied to the exact ADC configuration used in
 * ktool_alloc() (Scale2500 / Oversample64 / Samplingtime247_5). Change any of
 * those and the calibration must be redone.
 */
#define KTOOL_ADC_CHANNEL     FuriHalAdcChannel4
#define KTOOL_VS_DIVIDER_GAIN 16.085f /* 16.0 nominal x 1.00531 measured */
#define KTOOL_VS_OFFSET_MV    78.0f /* fixed offset of the measuring path */

/* VS is measured behind D1 (Schottky). Measured drop is ~117 mV at this load
 * (111 / 115 / 125 mV over 10-16 V) - far below the 300 mV datasheet figure,
 * which applies at ~1 A, not at the ~15 mA this circuit draws. */
#define KTOOL_D1_DROP_MV 117.0f

typedef enum {
    KToolEventTypeInput,
    KToolEventTypeTick,
} KToolEventType;

typedef struct {
    KToolEventType type;
    InputEvent input; /* valid only when type == KToolEventTypeInput */
} KToolEvent;

typedef struct {
    Gui* gui;
    ViewPort* view_port;
    FuriMessageQueue* event_queue;
    FuriTimer* timer;

    FuriHalAdcHandle* adc;

    /* Guards the sampled values below: written on the app thread, read on the GUI thread. */
    FuriMutex* mutex;
    uint16_t adc_raw;
    uint32_t vs_mv; /* voltage at the VS node, in millivolts */
} KTool;

#endif // KTOOL_I_H
