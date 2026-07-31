# Kextension — hardware

K-line / K-bus interface board for the Flipper Zero, based on two ST L9637D
transceivers. Designed for a Flipper Zero ProtoBoard v1.1, connected to the car
through OBD2.

The schematic is the source of truth for component designators; the BOM in the
main [README](../README.md) is kept in sync with it.

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

## Designators

The schematic is the source of truth; this table is what the BOM refers to.

| Ref | Part |
|---|---|
| **IC1** | L9637D on K-line — OBD pin 7 (DME), Flipper USART1 |
| **IC2** | L9637D on K-bus — OBD pin 8 (body), Flipper LPUART1 |
| **F1** | resettable fuse, PTC 650 mA / 60 V |
| **D1** | reverse-polarity diode, 1N5819 |
| **D2** | TVS, P6KE30A |
| **C4** | VS bulk capacitor, 10 µF / 50 V |
| **C1 / C7** | VCC decoupling, 100 nF (IC1 / IC2) |
| **C3 / C5** | VS decoupling, 100 nF (IC1 / IC2) |
| **C2 / C6** | K-line EMI filter, 1 nF / 1 kV (IC1 / IC2) |
| **R1 / R2** | K-line pull-up, 1 k (IC1 / IC2) |
| **R3 / R4** | VS-sense divider, 150 k / 10 k |
| **J1** | Flipper GPIO, left header |
| **J2** | Flipper GPIO, right header |
| **J3** | OBD2 breakout — 16, 5, 7, 8, 6, 14 (CAN pins broken out, not connected) |

## Conventions

- **Net naming:** underscore form in the schematic (`K_LINE`, `K_BUS`, `VS_SENSE`), hyphenated in prose (K-line, K-bus)
- **Ground:** single `GND` net, star topology — the transceiver, Flipper and bus grounds meet at one point. Place that point at the TVS anode / OBD ground entry, so surge current never shares copper with the Flipper's ground return
- **L9637D footprint:** stock `Package_SO:SOIC-8_3.9x4.9mm_P1.27mm`
