# Changelog

## 0.2.1

### Wine/Proton support

Wine's `winepulse.drv` rejects any sink with more than 18 channels, so the 22-channel pro-audio device cannot be used directly. A PipeWire stream rule now automatically routes all Wine/Proton audio to the 2-channel Main 1/2 loopback — no per-game launch options required. Audio is hardcoded to Main 1/2.

### Dynamic ALSA node discovery

The daemon now discovers ALSA node names from PipeWire at startup instead of using hardcoded names. The ALSA card index changes between reboots and USB re-enumerations, which previously caused loopbacks to silently fail to reach the hardware.

## 0.2.0

### Independent I/O selection

Output and input devices are now independently selectable in GNOME Settings. The daemon creates 7 virtual output devices and 5 virtual input devices, each routed to the correct physical I/O pair on the MOTU.

### WebSocket device control

Replaced MIDI SysEx transport with WebSocket over the MOTU's USB network interface. UDP multicast discovery finds the device automatically. Full device state sync (2400+ properties) on connect. Compatible with CueMix5 running simultaneously.

### Audio router

The daemon spawns and manages `pw-loopback` instances for each I/O pair using the pro-audio ALSA profile. Enforces sample rate via PipeWire metadata. Auto-restarts on failure. Router operates independently of device control — audio works even if the WebSocket connection fails.

### Dependencies

- Added: `tungstenite` (WebSocket), `socket2` (UDP discovery)
- Replaced: `midir` (MIDI) with WebSocket transport

## 0.1.1

- Set default device profile to `output:out-main` in WirePlumber rules so the MOTU always activates on detection, independent of WirePlumber's profile cache
- Clear stale WirePlumber profile cache on install/upgrade to prevent cached `off` state from overriding the default profile

## 0.1.0

- Initial release
- ALSA card profile with per-pair stereo output and input mappings
- WirePlumber integration (device naming, priority, profile set)
- udev rules for automatic device detection
- systemd user service for control daemon
- CLI tools: `motu-mk5d` (daemon), `motu-ctl` (device control)
- Packaging for Debian, Fedora (RPM), and Arch Linux
