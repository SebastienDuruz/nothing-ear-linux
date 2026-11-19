# Nothing earbuds control protocol

Audio uses the standard Bluetooth profiles (A2DP for music, HFP for the microphone), handled by BlueZ and PipeWire. No driver is needed.

Settings go through a **proprietary RFCOMM channel**. The earbuds advertise it under the UUID `aeac4a03-dff5-498f-843a-34487cf133eb`. On Nothing Ear (B171) it listens on channel **15**.

Nothing does not publish this protocol. This document lists:

- what was **verified** on a real pair of Nothing Ear (B171, firmware 1.0.1.54);
- what comes from community reverse engineering and is **still to be confirmed**.

## Frame format

| Bytes | Content |
|---|---|
| 0 | `0x55`, start of frame |
| 1–2 | Control field, u16 little-endian: `0x0160`. Bit `0x20` means the frame ends with a CRC |
| 3–4 | Command, u16 little-endian: high byte = direction, low byte = id |
| 5–6 | Payload length, u16 little-endian |
| 7 | Operation number, echoed back in the reply |
| 8… | Payload |
| last 2 | CRC-16/MODBUS, little-endian, over everything before it |

Direction (high byte of the command):

| Value | Meaning |
|---|---|
| `0xC0` | Read request from the computer; answered with `0x40` |
| `0xF0` | Write request from the computer; acknowledged with `0x70` |
| `0xE0` | Notification sent by the earbuds on their own |

A reply keeps the id of its request: `0xC007` (read battery) is answered by `0x4007`.

Code: [`crates/nothing-proto/src/frame.rs`](../crates/nothing-proto/src/frame.rs).

## Commands

✅ verified on B171 · 🔶 to be confirmed · ❌ no reply on B171

| Setting | Read | Write | Payload | State |
|---|---|---|---|---|
| Battery | `C007` | — | `n`, then `n` pairs (part, value). Part: 2 left, 3 right, 4 case. Value: bit 7 = charging, bits 0–6 = % | ✅ |
| Noise control | `C01E` | `F00F` | Read: triplets (kind, value, 0); kind 1 holds the mode. Write: `01 mode 00`. Mode: 1 high, 2 mid, 3 low, 4 adaptive, 5 off, 7 transparency | ✅ |
| EQ preset | `C01F` | `F010` | `preset 00`. 0 balanced, 3 voice, 5 custom equalizer; sources disagree on 1 and 2 (more bass / more treble) | ✅ read, ✅ 5, 🔶 1 and 2 |
| Custom EQ | `C044` | `F041` | `n`, overall gain (4 bytes), then 13 bytes per band: kind, gain, frequency (f32), Q (f32), then 9 zero bytes. Writing the bands does not select them: send `F010 05` afterwards. The read reply is longer (109 bytes on B171); only the first 44 bytes are meaningful | ✅ |
| Bass enhance | `C04E` | `F051` | `enabled level×2` (level 1 to 5) | ✅ read, 🔶 write |
| Low latency | `C041` | `F040` | 1 = on, 2 = off | ✅ |
| Options (incl. in-ear detection) | `C00E` | `F004` | Read: `n` pairs (id, value); id 1 = in-ear detection. Write: `01 id value` | ✅ read, 🔶 write |
| Gestures | `C018` | `F003` | Read: `n` entries of 4 bytes (side, 01, gesture, action). Write: `01 side 01 gesture action` | ✅ read, 🔶 write |
| Firmware | `C042` | — | ASCII text | ✅ |
| Device info | `C006` | — | Text lines `part,field,value`; field 2 = firmware, field 4 = serial number | ✅ |
| Ring | — | `F002` | `target 01` to start, `target 00` to stop; target 1 both, 2 left, 3 right | 🔶 |
| Ear tip fit test | `C00A` (prepare) | `F014` (`01`) | Result in `E00D` / `400D`: one byte per side, 0 = good seal | 🔶 |
| Personalized noise control | `C020` | `F011` | — | ❌ |

Gesture kinds: 2 double pinch, 3 triple pinch, 7 pinch and hold, 9 double pinch and hold. Sides: 2 left, 3 right.

Gesture actions: 1 none, 2 play/pause, 8 previous track, 9 next track, 0x0A noise control (cycle all), 0x0B voice assistant, 0x12 volume up, 0x13 volume down, 0x14 cancellation/off, 0x15 transparency/off, 0x16 cancellation/transparency.

Code: one file per setting in [`crates/nothing-proto/src/settings/`](../crates/nothing-proto/src/settings/), command codes in [`command.rs`](../crates/nothing-proto/src/command.rs).

## Linux connection quirks

With a non-blocking socket, `bluer` reports the RFCOMM connection as open before the kernel has finished setting it up. The very first reads and writes then fail with `ENOTCONN`. `nothing-link` retries them during the first three seconds (see `SETTLE_WINDOW` in [`client.rs`](../crates/nothing-link/src/client.rs)).

Right after a disconnection, the earbuds may also briefly refuse a new connection: several attempts are made (see `CONNECT_ATTEMPTS` in [`earbuds.rs`](../crates/nothing-link/src/earbuds.rs)).

## Confirming a "to be confirmed" command

The most reliable way is to capture what the official app sends:

1. On Android, enable **Developer options → Bluetooth HCI snoop log**.
2. Change the setting in the Nothing X app.
3. Get the log (from a bug report) and open it in Wireshark with the filter `btrfcomm`.

To try a command directly from Linux, `nothing-ctl raw` sends a raw frame and prints the reply:

```bash
nothing-ctl raw c01f
```
