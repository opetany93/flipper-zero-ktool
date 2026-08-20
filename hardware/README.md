# Kextension — hardware

K-line / K-bus interface board for the Flipper Zero, based on two ST L9637D
transceivers. Designed for a Flipper Zero ProtoBoard v1.1, connected to the car
through OBD2.

The schematic is the source of truth for designators and connectivity; the BOM
below lists the values those designators stand for.

## Files

| File | What |
|---|---|
| `kextension.kicad_pro` / `.kicad_sch` | KiCad project and schematic |
| `kextension-schematic.pdf` | exported schematic, readable without KiCad |

The schematic is self-contained: since KiCad 6 the symbols used on a sheet are
embedded in the `.kicad_sch` itself, so it opens correctly without the custom
symbol library it was drawn with. The custom symbols (`Conn_OBD2_BMW_6pin`,
`Conn_FlipperZero_left_pins`, `Conn_FlipperZero_right_pins`, `L9637`) live in a
personal library that is not part of this repository — you only need it to place
*new* instances of them. Footprints are stock KiCad.

## BOM

### Power and protection — one set, upstream of both transceivers

| Ref | Part | Value | Notes |
|---|---|---|---|
| **F1** | resettable fuse | PTC 650 mA / 60 V, THT | in series with OBD pin 16, ahead of D1 |
| **D1** | reverse-polarity diode | 1N5819 (Schottky, 40 V / 1 A) | cathode towards the board. Forward drop is **not** constant: 150 mV at 4 V to 199 mV at 16 V, following current — the app compensates for it |
| **D2** | TVS | P6KE30A (600 W) | cathode to VS, anode to GND. Clamps at ~41 V |
| **C4** | VS bulk capacitor | 10 µF / 50 V | ~10 mA load; 47 µF would be overkill |
| **R3** | VS-sense divider, upper | 150 k, 0.25 W, **metal film 1%** | VS → `VS_SENSE` node |
| **R4** | VS-sense divider, lower | 10 k, 0.25 W, **metal film 1%** | `VS_SENSE` node → GND, node also to PC3 |

The divider ratio depends on **both** resistors, so both are 1% metal film —
picked as much for temperature coefficient as for initial tolerance, since the
board lives in a car (−20…+60 °C).

### Per transceiver — IC1 (K-line, OBD 7) and IC2 (K-bus, OBD 8)

| Ref (IC1 / IC2) | Part | Value | Notes |
|---|---|---|---|
| **IC1 / IC2** | K-line transceiver | ST **L9637D**, SO-8 | on a SO-8 → DIP-8 adapter for the protoboard; the part is not made in DIP |
| **C1 / C7** | VCC decoupling | 100 nF / 50 V X7R | VCC ↔ GND |
| **C3 / C5** | VS decoupling | 100 nF / **50 V** | VS ↔ GND. Must be 50 V, not 16 V — 14.5 V of operating voltage plus MLCC DC bias |
| **C2 / C6** | K-line EMI filter | 1 nF / 1 kV | K ↔ GND |
| **R1 / R2** | K-line pull-up | 1 k, 0.25 W | K ↔ VS. Dissipates ~0.18 W worst case at 13.5 V |

**Pull-up value.** 1 k is not the only workable choice — the MikroE Click
reference board uses 4.7 k, and 510 Ω gives sharper edges on a long cable. At
the 10.4 kbaud of KWP2000 all three work.

**VS decoupling** (C3 / C5) is good practice rather than a requirement — the
Click board omits it. It is populated on both chips here because in a car VS is
a dirty rail.

### Connectors

| Ref | Part | Notes |
|---|---|---|
| **J1** | Flipper GPIO, left header | ground comes in on pin 8 |
| **J2** | Flipper GPIO, right header | both GND pins are no-connect |
| **J3** | OBD2 breakout | pins 16, 5, 7, 8, 6, 14 — the CAN pins are broken out but not connected |

Unused L9637D pins: **LI** (pin 2) and **LO** (pin 8) are no-connect on both
chips — the L-line is not used, KWP2000 runs on K alone.

## Conventions

- **Net naming:** underscore form in the schematic (`K_LINE`, `K_BUS`, `VS_SENSE`), hyphenated in prose (K-line, K-bus)
- **Ground:** single `GND` net, star topology — the transceiver, Flipper and bus grounds meet at one point. Place that point at the TVS anode / OBD ground entry, so surge current never shares copper with the Flipper's ground return
- **L9637D footprint:** stock `Package_SO:SOIC-8_3.9x4.9mm_P1.27mm`
