# motu-mk5-linux

Native Linux integration for the MOTU UltraLite mk5 audio interface.

**Status: Work in Progress** — Audio I/O and system integration are working. Device control plane (mixer, EQ, routing) is under development.

---

## For Users

### What this does

Installs a single `.deb` package that makes the MOTU UltraLite mk5 work natively on Linux. No manual configuration, no workarounds.

After install:
- All physical I/O pairs appear as selectable profiles in GNOME Sound Settings
- Speaker test buttons work (Front Left / Front Right)
- Direct audio path to hardware — no loopback layers, no resampling
- Device is auto-detected on USB plug-in
- Survives reboots and system updates

### Requirements

- PipeWire + WirePlumber
- MOTU UltraLite mk5

### Install

See [Releases](https://github.com/MarkWind85/motu-mk5-linux/releases) for install packages and instructions.

### Available profiles

Select these in GNOME Sound Settings under Configuration:

**Output**
| Profile | Physical output |
|---|---|
| Main 1/2 | Main output pair (back panel) |
| Line Out 3/4 | Analog line out 3-4 |
| Line Out 5/6 | Analog line out 5-6 |
| Line Out 7/8 | Analog line out 7-8 |
| Line Out 9/10 | Analog line out 9-10 |
| Phones | Headphone jack |
| S/PDIF Out | S/PDIF digital output |
| All Outputs | All 22 channels (for DAW use) |

**Input**
| Profile | Physical input |
|---|---|
| Mic/Line 1/2 | Combo jacks (mic or line level) |
| Line In 3/4 | Analog line in 3-4 |
| Line In 5/6 | Analog line in 5-6 |
| Line In 7/8 | Analog line in 7-8 |
| S/PDIF In | S/PDIF digital input |
| All Inputs | All 20 channels (for DAW use) |

Output and input profiles are independent — pick any output with any input.

### MIDI

MIDI support is coming.

### CLI tool

```bash
# List all device properties
motu-ctl list

# Probe and identify connected device
motu-ctl probe
```

> Note: `motu-ctl` device control (gain, EQ, mixer, routing) requires the control plane which is under development. The CLI currently builds and connects but the mk5's MIDI SysEx response path needs further investigation.

---

## For Developers

### Architecture

The mk5 has two planes on USB:

1. **Audio streaming** — USB Audio Class 2.0, handled by the Linux kernel's `snd-usb-audio` driver. Works out of the box. 22 output / 20 input channels at 48kHz.

2. **Control plane** — Mixer, EQ, compressor, gate, reverb, routing, preamp gain, 48V phantom power. This is what CueMix5 provides on macOS/Windows. On Linux, it's what this project implements.

### What the package installs

| File | Path | Purpose |
|---|---|---|
| `motu-ultralite-mk5.conf` | `/usr/share/alsa-card-profile/mixer/profile-sets/` | ALSA Card Profile — maps physical I/O pairs to stereo FL/FR channels |
| `51-motu-mk5.lua` | `/usr/share/wireplumber/main.lua.d/` | WirePlumber rules — device naming, priority, profile set reference |
| `89-motu-mk5.rules` | `/etc/udev/rules.d/` | udev — device detection, ACP profile set assignment, daemon trigger |
| `motu-mk5d.service` | `/usr/lib/systemd/user/` | systemd user service for the control daemon |
| `motu-mk5d` | `/usr/bin/` | Control daemon — connects via MIDI SysEx, syncs/persists device state |
| `motu-ctl` | `/usr/bin/` | CLI tool — read/write any device parameter |

### Control protocol

The mk5's control protocol was extracted from the CueMix5 Electron app source (`/opt/com.motu.CueMix5-1.0.0/resources/app.asar`). Key source files:

- `datastore.js` — Binary property protocol (encode/decode, send/receive)
- `midi.js` — MIDI SysEx transport layer, 7-bit encoding
- `dev.js` — UltraLite mk5 property definitions (all IDs, types, value ranges)
- `dev_common.js` — Shared constants and types

**Transport:** MIDI SysEx over USB MIDI interface (interface 4)
- MOTU manufacturer ID: `00 00 3B`
- Protocol ID: `00 01`
- Request types: `SetProperty(0)`, `ProtocolProbe(1)`, `EnableSysexAPI(2)`

**Binary property format:**

```
Host → Device: [prop_id:u16] [index:u16] [length:u16] [data]
Device → Host: [prop_id:u16] [index:u16] [data]
```

- Floats use 8.24 fixed-point encoding (`value * 0x01000000`)
- Strings are 32-byte null-terminated
- All multi-byte values are big-endian

**Connection flow:**
1. Send `ProtocolProbe` SysEx
2. Device responds confirming MOTU protocol
3. Send `EnableSysexAPI`
4. Device streams all current property values
5. Send `SetProperty` to change any parameter

### Property map

60+ properties covering the full device feature set. Run `motu-ctl list` or see `src/protocol/properties.rs` for the complete map. Major categories:

| Category | Property IDs | Description |
|---|---|---|
| Device info | 0–21 | MAC, UID, name, firmware, model, API version |
| Clock | 10–16 | Sample rate, clock source, lock status |
| Preamp | 5001–5005 | Gain (0–74dB mic, 0–20dB line), 48V, pad, phase |
| Routing | 5010, 5014 | FPGA patch table (80 entries), loopback source |
| Output | 5000, 5011–5012 | Per-output trim, main group bitmask |
| Names | 6, 7 | Input/output channel names |
| Mixer | 1016–1019 | 32-input x 14-bus matrix: fader, pan, solo, mute |
| Input EQ | 1002–1006 | 4-band parametric: mode, freq, gain, bandwidth |
| Output EQ | 1022–1026 | 3-band parametric per bus |
| Gate | 1007–1010 | Threshold, attack, release (mic inputs) |
| Compressor | 1011–1015, 1021 | Threshold, attack, release, ratio, makeup |
| Reverb | 1030–1034 | Decay, damping, pre-delay, width, preset |
| Talkback | 5026–5031 | Enable, level, dim, source, destination |
| Meters | 6000 | All level meters |

### Open questions

- **MIDI SysEx**: The probe message is sent and received by the device (ALSA shows Tx bytes incrementing) but no response comes back. Either the mk5 firmware requires activation, or the SysEx property protocol isn't available on this model over MIDI. CueMix5's WebMIDI code path suggests it should work.

- **CDC Ethernet**: The mk5 exposes a USB CDC Ethernet interface (`enx*`) that responds to ping at a link-local address, but no TCP ports are open. The MOTU AVB HTTP API (documented by MOTU at port 80/1280) isn't served over this link.

- **USB vendor interface**: Interface 7 has an interrupt IN endpoint only (no OUT). May carry meter data. The Drumfix `motu-avb-usb` project has partial documentation of the vendor USB protocol in `protocol.h`.

### Building from source

```bash
# Requires: Rust toolchain, libasound2-dev
cargo build --release
cargo test

# Build .deb package
cargo install cargo-deb
cargo deb
```

### Project structure

```
src/
  protocol/
    sysex.rs        — MIDI SysEx framing (encode/decode/build/parse)
    properties.rs   — Full property map (60+ properties)
    types.rs        — Property types and 8.24 fixed-point conversion
  device/
    connection.rs   — MIDI port discovery and I/O
    state.rs        — Device state manager (sync, persist, restore)
  daemon.rs         — motu-mk5d entry point
  ctl.rs            — motu-ctl entry point
  lib.rs
install/
  alsa-card-profile/  — ACP profile for channel mapping
  wireplumber/        — WirePlumber node configuration
  udev/               — Device detection rules
  systemd/            — User service definition
```

## License

GPL-2.0-or-later
