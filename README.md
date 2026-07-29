# KTool

K-line / K-bus diagnostics on the Flipper Zero.

The project has two halves:

- **KTool** — the Flipper application (this repository's `.c` / `.h` files)
- **Kextension** — the hardware it talks through: a Flipper Zero GPIO board built around two **ST L9637D** transceivers, plugged into the car through the OBD2 connector. Schematic and BOM live in [`hardware/`](hardware/).

Originally developed against a **BMW E46 (330Ci, 2001)**, but K-line is not a BMW thing — the same physical layer (ISO 9141-2 / ISO 14230 KWP2000) was used by VW/Audi, Mercedes, PSA, Renault, Fiat, Opel and others until CAN took over. Hence no model name in the app.

> ⚠️ **Early work in progress.** The app currently brings up the GUI and reads the supply voltage. Diagnostics, live data and K-bus support are not implemented yet. The hardware side is designed and verified on the bench, but not yet assembled on a protoboard.

---

## Status

| Area | State |
|---|---|
| App skeleton (GUI, event loop, timer) | ✅ working |
| VS-sense (supply voltage over ADC) | ✅ working, calibrated to ±7 mV |
| Dual UART transport (K-line + K-bus) | ⬜ not started — no code yet. Approach validated on paper: both ports are exposed by `furi_hal_serial`, so no multiplexing hardware is needed |
| Schematic | ✅ complete (KiCad) |
| Hardware assembly | ⬜ not started |
| KWP2000 / DS2 diagnostics | ⬜ not started |
| K-bus sniffing | ⬜ not started |

---

## Planned features

- **Fault codes** — read and clear DTCs
- **Gauge** — live data: RPM, coolant and oil temperature, supply voltage, fuel trims
- **Actuators** — actuator tests (pump, relays, injectors)
- **Adaptations** — adaptation reset, flagged as an advanced operation
- **ECU info** — controller identification
- **0-100 km/h** — acceleration timing over OBD-II PID `0x0D` (indicative only, see note below)
- **Comfort close** — roll the windows up over the body bus with a double click

---

## Hardware — Kextension

Two L9637D transceivers, one per bus, each on its own UART. No multiplexing, no select line, and both buses can be listened to at the same time.

Board: Flipper Zero ProtoBoard v1.1. KiCad project, schematic PDF and BOM are in [`hardware/`](hardware/).

### Pin mapping

| OBD2 pin | Signal | Flipper | Peripheral |
|---|---|---|---|
| 16 | B+ (permanent 12 V) | — | via F1 → D1 → VS |
| 4 / 5 | GND | 11, 18 | common ground is mandatory |
| **7** | K-line (DME) — `D_TXD2` | 13 / 14 (PB6 / PB7) | USART1 |
| **8** | K-bus (body) — `D_TXD1` | 15 / 16 (PC1 / PC0) | LPUART1 |
| 6 / 14 | CAN-H / CAN-L | — | broken out, not connected |
| — | VS-sense divider | 7 (PC3) | ADC1_IN4 |

⚠️ OBD2 pin 8 is **not populated on every E46**. If it reads dead, `D_TXD1` can be picked up at `X11175` pin 25 (white/violet, right instrument cluster connector) or on the round 20-pin diagnostic connector, pin 20.

### Power and protection

```
OBD pin 16 -> F1 (PTC 650 mA / 60 V) -> D1 (1N5819) -+-> VS
                                                     +-> D2 (P6KE30A TVS)
                                                     +-> C4 (10 uF)
                                                     +-> R3 150k -+- R4 10k -> GND
                                                                  |
                                                                  +-> PC3 (ADC)
```

- **Logic runs at 3.3 V.** Flipper GPIO is **not 5 V tolerant** — do not power the transceiver's VCC from 5 V.
- **The Flipper is never powered from the car.** 12 V stays on the transceiver side; only 3.3 V logic crosses over.
- The TVS clamps at ~41 V, which also keeps the sense divider below the 3.3 V pin limit during a load dump.

### Supply voltage sensing

A 150k / 10k divider off the VS rail feeds PC3. The app samples it every 500 ms with:

```c
furi_hal_adc_configure_ex(adc, FuriHalAdcScale2500, FuriHalAdcClockSync64,
                          FuriHalAdcOversample64, FuriHalAdcSamplingtime247_5);
```

The long sampling time is **not optional**: the divider presents ~9.4 kΩ to the ADC, and a short sampling window leaves the sample-and-hold capacitor undercharged — producing a reading that is low but perfectly stable, and therefore easy to mistake for a correct one.

Calibration constants in `ktool_i.h` were fitted against a multimeter at 10 / 14 / 16 V and land within **±7 mV** across that range. They are tied to the exact ADC configuration above — change `Scale`, `Oversample` or `Samplingtime` and the calibration must be redone.

---

## Building

Built with [ufbt](https://github.com/flipperdevices/flipperzero-ufbt):

```sh
pipx install ufbt
ufbt              # build -> dist/ktool.fap
ufbt launch       # build, install to the SD card, run
ufbt vscode_dist  # VS Code config (IntelliSense + tasks)
```

`ufbt launch` installs the app permanently to `/ext/apps/Tools/ktool.fap`; it shows up under **Apps → Tools → KTool**.

### Required Flipper settings

These reset to their defaults after a firmware reinstall, so check them before connecting hardware:

| Setting | Value | Why |
|---|---|---|
| System → **Log Device** | `None` | The default is USART — pins 13/14 — which would spray log text straight into the K-line |
| System → **Log Level** | `Info` | Often defaults to `none`, in which case `log` in the CLI stays silent |
| **Expansion Modules** → Listen UART | disabled | The service occupies whichever UART it listens on, and `furi_hal_serial_control_acquire()` then returns NULL |

Logs are read over USB, which keeps both UARTs free:

```sh
ufbt cli
log info
```

---

## ⚠️ Safety

This tool talks to a car. Read this before connecting it to anything that moves.

- **Bus sleep.** The interface must go quiet and release the line after every action. A body bus that never sleeps will flatten the battery in a couple of days. This is the single most common failure in home-made K-bus adapters.
- **No arbitration.** Unlike CAN, the K-bus has none — transmitting into the middle of someone else's frame corrupts both. Only transmit during silence.
- **No anti-pinch.** E46 windows have no anti-pinch protection; the factory comfort-close requires the driver to hold the key precisely because someone is watching. Any automated version must be constrained to a locked, empty car.
- **Adaptation resets are not reversible by undo.** The ECU relearns from scratch afterwards — expect a rough idle and a searching gearbox for a while.
- **Acceleration timing is indicative.** K-line runs at 10.4 kbaud request/response, giving roughly 5–15 samples per second at 1 km/h resolution, plus a few hundred milliseconds of ECU filtering latency. Expect errors of a few tenths of a second. This is not a Dragy.

Use at your own risk.

---

## Compatibility

K-line era BMWs: **E36, E46, E39, E38, E53**. Newer models (E9x/E6x onward, F and G series) dropped K-line in favour of CAN/DoIP and will not respond.

OBD2 is a **connector** standard (SAE J1962), not a protocol one — the socket fitting does not mean the protocol is there.

---

## Roadmap

1. Runtime check that `furi_hal_serial_control_acquire(FuriHalSerialIdLpuart)` succeeds
2. Assemble on a Flipper Zero ProtoBoard v1.1
3. Confirm whether a live K-bus is present on OBD2 pin 8 on the target car
4. KWP2000 transport layer, then fault codes
5. K-bus sniffer (receive only) before any transmission
6. Comfort close
7. CAN — deferred; needs either bxCAN plus an SN65HVD230, or an MCP2515 module (avoid TJA1050 variants, they are 5 V only)

---

## Repository layout

```
ktool.c, ktool_i.h    application sources
application.fam       FAP manifest
images/               icon assets
hardware/             Kextension: KiCad project, schematic PDF, BOM
```

## License

TBD.
