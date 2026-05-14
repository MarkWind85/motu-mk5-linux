# motu-mk5-linux

Native Linux integration for the MOTU UltraLite mk5 audio interface.

**Status: Work in Progress**

## What this does

Provides full device control for the MOTU UltraLite mk5 on Linux — mixer, EQ, compressor, gate, reverb, routing, preamp gain, 48V phantom power, and more. Installs as a system service that auto-starts when the mk5 is plugged in and persists your configuration across reboots.

Audio streaming uses the standard Linux `snd-usb-audio` kernel driver (USB Audio Class 2.0). This project adds the **control plane** that MOTU only officially supports via CueMix5 on macOS/Windows.

## How it works

The mk5 exposes a MIDI SysEx control protocol alongside its standard USB audio interface. This project implements that protocol to provide:

- **`motu-mk5d`** — daemon that manages device state, restores settings on connect, and persists changes
- **`motu-ctl`** — CLI tool to read/write any device parameter
- **WirePlumber integration** — proper PipeWire device/node naming and priority
- **udev + systemd** — automatic startup on USB attach

## Install

```
make build
sudo make install
```

Requires: Rust toolchain, ALSA development libraries (`libasound2-dev` on Debian/Ubuntu).

## Usage

```bash
# Show device info
motu-ctl probe

# Show current status
motu-ctl status

# List all controllable properties
motu-ctl list

# Set mic 1 gain to 45 dB
motu-ctl set input_gain -i 0 45

# Enable 48V phantom power on mic 1
motu-ctl set input_48v -i 0 1

# Set sample rate
motu-ctl set sample_rate 96000

# Dump full device state as JSON
motu-ctl dump
```

## Uninstall

```
sudo make uninstall
```

## Supported features

| Feature | Status |
|---|---|
| Input gain / 48V / pad / phase | Implemented |
| Internal mixer (32x14 matrix) | Implemented |
| Per-channel EQ (4-band parametric) | Implemented |
| Compressor / Gate | Implemented |
| Reverb | Implemented |
| Output routing / trim | Implemented |
| Channel naming | Implemented |
| Sample rate / clock source | Implemented |
| State persistence | Implemented |
| Auto-start on USB attach | Implemented |
| PipeWire integration | Implemented |
| Talkback | Implemented |
| Metering | Not yet |
| ADAT/S-PDIF optical mode | Implemented |

## Protocol

The control protocol was documented by studying the CueMix5 application source. Communication uses MIDI SysEx messages over the mk5's USB MIDI interface with MOTU manufacturer ID `00 00 3B` and protocol ID `00 01`.

## License

GPL-2.0-or-later
