# motu-mk5-linux

Native Linux integration for the MOTU UltraLite mk5 audio interface.

For mixer, EQ, and routing control, use [CueMix5 for Linux](https://github.com/MarkWind85/com.motu.CueMix5-1.0.0).

---

## For Users

### What this does

Makes the MOTU UltraLite mk5 work natively on Linux. No manual configuration, no workarounds.

After install:
- All physical I/O pairs appear as separate devices in GNOME Sound Settings
- Output and input are independently selectable — pick any output with any input
- Device is auto-detected on USB plug-in
- Survives reboots and system updates
- Full device control via CLI (`motu-ctl`)

### Available I/O

Select these in GNOME Sound Settings under Output Device / Input Device:

**Outputs**
| Device | Physical output |
|---|---|
| MOTU Main 1/2 | Main output pair (back panel) |
| MOTU Line 3/4 | Analog line out 3-4 |
| MOTU Line 5/6 | Analog line out 5-6 |
| MOTU Line 7/8 | Analog line out 7-8 |
| MOTU Line 9/10 | Analog line out 9-10 |
| MOTU Phones | Headphone jack |
| MOTU S/PDIF Out | S/PDIF digital output |

**Inputs**
| Device | Physical input |
|---|---|
| MOTU Mic/Line 1/2 | Combo jacks (mic or line level) |
| MOTU Line In 3/4 | Analog line in 3-4 |
| MOTU Line In 5/6 | Analog line in 5-6 |
| MOTU Line In 7/8 | Analog line in 7-8 |
| MOTU S/PDIF In | S/PDIF digital input |

### Wine / Proton

Wine and Proton games work out of the box. Audio is automatically routed to the Main 1/2 output — no launch options or per-game configuration needed.

**Why this is needed:** Wine's PulseAudio driver rejects any audio device with more than 18 channels. The MOTU's pro-audio profile exposes 22 channels, so Wine can't use it directly. The package installs a PipeWire stream rule that routes all Wine/Proton audio to the 2-channel Main 1/2 stereo loopback instead.

**Note:** Wine audio is hardcoded to Main 1/2. If you need Wine audio on a different output pair, edit `~/.config/pipewire/pipewire-pulse.conf.d/50-motu-wine-routing.conf` and change `target.object` to the desired sink name (e.g., `motu-phones` for the headphone output).

### Requirements

- PipeWire + WirePlumber
- MOTU UltraLite mk5

### Install

See [Releases](https://github.com/MarkWind85/motu-mk5-linux/releases) for install packages and instructions.

### CLI tool

```bash
# Show device status (model, sample rate, gains, clock)
motu-ctl status

# Get a property
motu-ctl get sample_rate

# Set a property
motu-ctl set input_gain --index 0 40

# List all device properties
motu-ctl list

# Probe and identify connected device
motu-ctl probe

# Dump full device state as JSON
motu-ctl dump

# Generate a diagnostic report for troubleshooting
motu-ctl diagnose

# Check for updates
motu-ctl update --check

# Update to latest release
motu-ctl update
```

### Troubleshooting

**Check daemon status:**
```bash
systemctl --user status motu-mk5d
```

**View live logs:**
```bash
journalctl --user-unit motu-mk5d -f
```

**Generate a diagnostic report:**
```bash
motu-ctl diagnose
```
This collects system info, PipeWire state, device connection status, and recent logs in one command. Paste the output into a [GitHub issue](https://github.com/MarkWind85/motu-mk5-linux/issues/new/choose) if you need help.

**Common issues:**

| Problem | Likely cause | Fix |
|---|---|---|
| No MOTU devices in sound settings | PipeWire not running or profile not set | `systemctl --user restart pipewire wireplumber` |
| "device not available" in logs | USB not connected or CDC Ethernet missing | Check `ip link` for a 169.254.x.x interface |
| Audio crackling/dropouts | Sample rate mismatch or CPU load | Check `motu-ctl get sample_rate`, close heavy apps |
| Daemon keeps restarting | PipeWire not ready at boot | Check logs — daemon retries automatically |

---

## For Developers

### Architecture

The daemon (`motu-mk5d`) manages two independent subsystems:

1. **Audio router** — Spawns `pw-loopback` instances to create virtual stereo devices for each physical I/O pair. Uses the pro-audio ALSA profile (single device open, all channels) with per-pair channel routing. Independent of device control — audio works even if the control connection fails.

2. **Device control** — Connects to the MOTU over WebSocket (`ws://device:1280`) via its USB network interface. UDP multicast discovery finds the device automatically. Full bidirectional property sync — reads device state on connect, persists to disk, restores on reconnect.

### What the package installs

| File | Path | Purpose |
|---|---|---|
| `motu-ultralite-mk5.conf` | `/usr/share/alsa-card-profile/mixer/profile-sets/` | ALSA Card Profile — channel mapping definitions |
| `51-motu-mk5.lua` | WirePlumber config dir | Sets pro-audio profile on the MOTU device |
| `89-motu-mk5.rules` | `/etc/udev/rules.d/` | udev — device detection, profile set assignment |
| `motu-mk5d.service` | systemd user service dir | Control daemon auto-start |
| `motu-mk5d` | `/usr/bin/` | Daemon — audio router + WebSocket device control |
| `motu-ctl` | `/usr/bin/` | CLI tool — read/write any device parameter |
| `50-motu-wine-routing.conf` | `~/.config/pipewire/pipewire-pulse.conf.d/` | Routes Wine/Proton audio to Main 1/2 |

### Control protocol

The mk5 exposes a USB CDC Ethernet interface with a WebSocket server on port 1280.

**Discovery:** The device broadcasts JSON on UDP multicast port 1280:
```json
{"uid":"ULM5FFE434","name":"UltraLite-mk5","ip":"169.254.53.228","model":"UltraLite-mk5","version":"2.0.8+2568","interval":1}
```

**Transport:** WebSocket binary frames at `ws://device-ip:1280`

**Binary property format:**
```
Send/Receive: [prop_id:u16] [index:u16] [data]
```

- All multi-byte values are big-endian
- Floats use 8.24 fixed-point encoding (`value * 0x01000000`)
- Strings are 32-byte null-terminated

**Connection flow:**
1. Listen for UDP discovery broadcast on port 1280
2. Connect WebSocket to device IP
3. Device immediately streams all current property values
4. Send binary property frames to change parameters

### Property map

95+ properties covering the full device feature set. Run `motu-ctl list` or see `src/protocol/properties.rs` for the complete map. Major categories:

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

### Building from source

```bash
# Requires: Rust toolchain, libpipewire-0.3-dev
cargo build --release

# Build .deb package
cargo install cargo-deb
cargo deb
```

### Project structure

```
src/
  audio/
    discovery.rs    — ALSA node discovery from PipeWire
    router.rs       — pw-loopback management (spawn, stop, rate enforcement)
  protocol/
    sysex.rs        — MIDI SysEx framing (legacy, kept for reference)
    properties.rs   — Full property map (95+ properties)
    types.rs        — Property types and 8.24 fixed-point conversion
  device/
    connection.rs   — UDP discovery + WebSocket transport
    state.rs        — Device state manager (sync, persist, restore)
  diagnostics.rs    — System diagnostic report generation
  updater.rs        — Self-update from GitHub releases
  daemon.rs         — motu-mk5d entry point
  ctl.rs            — motu-ctl entry point
  lib.rs
install/
  alsa-card-profile/  — ACP profile for channel mapping
  pipewire-pulse/     — Wine/Proton audio routing rule
  wireplumber/        — WirePlumber device configuration
  udev/               — Device detection rules
  systemd/            — User service definition
```

## License

GPL-2.0-or-later
