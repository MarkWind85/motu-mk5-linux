# Changelog

## 0.4.0

### Installation error handling

- **postinst phased execution**: Install script now reports per-user failures individually instead of silencing all errors. Critical config installs (WirePlumber, PipeWire pulse) report warnings on failure. Audio stack restart failures are reported per-user.
- **Post-install validation**: After install, verifies that binaries are on PATH, ALSA profile exists, and udev rules are in place. Reports warnings for any missing components instead of always claiming success.
- **Makefile preflight checks**: `make install` now checks for required tools (`pw-loopback`, `pw-cli`, `systemctl`) before building. Warns if optional tools (`pactl`, `pw-metadata`) are missing. Verifies PipeWire and WirePlumber restarted successfully after install.

## 0.3.0

### Runtime error handling

- **WebSocket disconnect detection**: The daemon now detects when the device disconnects and automatically reconnects. Previously, a dropped USB connection left the daemon running but silently doing nothing — device state changes were lost, and stale state was pushed back on restart.
- **ALSA discovery retry**: Node discovery retries up to 10 times with exponential backoff (1s–16s) instead of exiting immediately. The daemon continues without audio routing if all attempts fail, so device control still works during PipeWire startup delays.
- **Corrupted state recovery**: Corrupted state files are now logged, backed up to `device-state.json.corrupt`, and the daemon continues with default state instead of silently discarding saved settings.
- **Atomic audio router start**: Loopback processes are now started atomically — if any channel fails to spawn, all already-started channels are cleaned up. No more partial audio routing with missing channels.
- **Command error visibility**: `pw-metadata` and `pactl` failures are now logged with stderr output instead of being silently ignored.
- **Signal handler logging**: Signal registration failures are logged instead of silently ignored.
- **Systemd restart limits**: The service now stops restarting after 5 failures in 60 seconds instead of crash-looping indefinitely.

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
