# KTool

K-line / K-bus diagnostics on the Flipper Zero.

The project has two halves:

- **KTool** — the Flipper application (Rust, in [`src/`](src/))
- **Kextension** — the hardware it talks through: a Flipper Zero GPIO board built around two **ST L9637D** transceivers, plugged into the car through the OBD2 connector. Schematic and BOM live in [`hardware/`](hardware/).

Originally developed against a **BMW E46 (330Ci, 2001)**, but K-line is not a BMW thing — the same physical layer (ISO 9141-2 / ISO 14230 KWP2000) was used by VW/Audi, Mercedes, PSA, Renault, Fiat, Opel and others until CAN took over. Hence no model name in the app.

> ⚠️ **Early work in progress.** The app currently brings up the GUI and reads the supply voltage. Diagnostics, live data and K-bus support are not implemented yet. The hardware is assembled on a protoboard but not yet tested against a car.

---

## Status

| Area | State |
|---|---|
| App skeleton (GUI, event loop, timer) | ✅ working |
| VS-sense (supply voltage over ADC) | ✅ working, calibrated to ±7 mV |
| Dual UART transport (K-line + K-bus) | ⬜ not started — no code yet. Approach validated on paper: both ports are exposed by `furi_hal_serial`, so no multiplexing hardware is needed |
| Schematic | ✅ complete (KiCad) |
| Hardware assembly | 🟨 assembled on a protoboard, not yet tested |
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

A 150k / 10k divider off the VS rail feeds PC3. The app samples it every 500 ms with the `HIGH_IMPEDANCE_2V5` profile in [src/hal/adc.rs](src/hal/adc.rs):

```rust
scale:         FuriHalAdcScale2500,
clock:         FuriHalAdcClockSync64,
oversample:    FuriHalAdcOversample64,
sampling_time: FuriHalAdcSamplingtime247_5,
```

The long sampling time is **not optional**: the divider presents ~9.4 kΩ to the ADC, and a short sampling window leaves the sample-and-hold capacitor undercharged — producing a reading that is low but perfectly stable, and therefore easy to mistake for a correct one.

The four settings are exposed only as a whole named profile, never as independent knobs, because the calibration below is fitted against that exact combination.

Calibration constants in [src/sensor/calibration.rs](src/sensor/calibration.rs) were fitted against a multimeter at 10 / 14 / 16 V and land within **±7 mV** across that range. They are tied to the ADC profile above — change `scale`, `oversample` or `sampling_time` and the calibration must be redone.

---

## Building

The app is written in Rust against [flipperzero-rs](https://github.com/flipperzero-rs/flipperzero). The toolchain channel and target are pinned in `rust-toolchain.toml`, so `rustup` installs the right ones on the first build.

```sh
cargo build --release   # -> target/thumbv7em-none-eabihf/release/ktool.fap
```

Nightly is required, but only for the `different-binary-name` cargo feature — that is what lets the binary be emitted as `ktool.fap` rather than a bare ELF.

Copy the `.fap` to `/ext/apps/Tools/ktool.fap` on the SD card (qFlipper, or `ufbt` if you have it installed for other reasons); it shows up under **Apps → Tools → KTool**.

The application icon is committed as `src/ktool.icon`, since there is no fbt in a Cargo build to convert the PNG. Regenerate it after editing `ktool.png`:

```sh
./tools/png-to-icon.ps1 -Source ktool.png -Destination src/ktool.icon
```

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
2. Bench-test the assembled board: supply rails, VS-sense divider, transceiver idle levels
3. Confirm whether a live K-bus is present on OBD2 pin 8 on the target car
4. KWP2000 transport layer, then fault codes
5. K-bus sniffer (receive only) before any transmission
6. Comfort close
7. CAN — deferred; needs either bxCAN plus an SN65HVD230, or an MCP2515 module (avoid TJA1050 variants, they are 5 V only)

---

## Repository layout

```
src/main.rs           FAP manifest, entry point, composition root
src/app.rs            event loop and the state shared with the GUI thread
src/ui.rs             frame layout; pure drawing
src/event.rs          the queue the event loop drains
src/sensor/           physical quantities
  mod.rs                SupplyReading and the SupplyVoltageSource trait
  calibration.rs        the fitted correction, pure arithmetic
  vs_divider.rs         the ADC-backed implementation
src/hal/              safe RAII wrappers over the Furi C API
  adc.rs, canvas.rs, input.rs, timer.rs, view_port.rs
src/text.rs           heap-free formatting for the C drawing API
src/units.rs          Millivolts
tools/                icon conversion
hardware/             Kextension: KiCad project, schematic PDF, BOM
```

Dependencies point one way only — `app`/`ui` → `sensor` → `hal` — and `unsafe` appears nowhere outside `src/hal/`. The event loop depends on the `SupplyVoltageSource` trait rather than on the ADC, so the day supply voltage starts arriving over KWP2000 instead, only `main.rs` changes.

Shutdown ordering is enforced rather than documented: the timer, the view port and the shared state are ordinary values whose `Drop` order the borrow checker fixes, so the timer provably stops before the queue it posts into goes away.

## License

GNU General Public License v3.0. See [`LICENSE`](LICENSE).
