# Changelog

## 0.5.1

- **Fix ALSA sink volume drift**: WirePlumber's stream restore was saving and re-applying volume changes on the pro-audio ALSA sink, causing it to drift from 100% after device reconnects or desktop volume adjustments. The WirePlumber config now sets `state.restore-props = false` on MOTU ALSA nodes so volume is managed exclusively by the hardware. Cleaned up stale state entries from previous card enumerations that contained poisoned default volumes.

## 0.5.0

Three releases worth of work shipped together: runtime resilience, installation reliability, and user-facing diagnostics. Updating from 0.2.1 to 0.5.0 gives you all of the below.

### Runtime error handling

- **WebSocket disconnect detection and reconnect**: The daemon now detects when the MOTU loses its USB connection and automatically reconnects when it comes back. Previously, a dropped connection left the daemon running but silently doing nothing — device state changes made on the hardware were lost, and on the next daemon restart, stale saved state was pushed back to the device, overwriting whatever the user had changed.
- **ALSA discovery retry with backoff**: At startup, PipeWire node discovery now retries up to 10 times with exponential backoff (1s, 2s, 4s, ... up to 16s) instead of exiting immediately on failure. This handles the common boot race where the daemon starts before PipeWire finishes enumerating nodes. If all attempts fail, the daemon continues without audio routing — device control still works, and the user sees a clear error explaining what happened.
- **Corrupted state file recovery**: If the JSON state file is corrupted (e.g., from a power loss during write), it's now logged at `warn!` level, backed up to `device-state.json.corrupt`, and the daemon starts with default state. Previously, corrupted files were silently discarded with no log entry — users lost all saved settings without knowing why.
- **Atomic audio router start**: When spawning the 12 `pw-loopback` processes, they now start all-or-nothing. If any one fails, all already-started processes are killed and a clear error is reported. Previously, a failure partway through left some channels working and others missing, with no indication which or why.
- **External command error visibility**: `pw-metadata` (sample rate enforcement) and `pactl` (ALSA sink volume) now capture stderr and log failures at `warn!` level. Previously, both commands discarded all output and ignored exit status — if either tool was missing or failed, the user had no way to know.
- **Signal handler logging**: If SIGINT/SIGTERM handler registration fails (possible in containers or restricted environments), the failure is now logged instead of silently ignored.
- **Systemd restart limits**: The service file now includes `StartLimitBurst=5` and `StartLimitIntervalSec=60`, so a crash-looping daemon stops after 5 failures in 60 seconds instead of restarting indefinitely. Users can then see the failure via `systemctl --user status motu-mk5d`.

### Installation error handling

- **Phased postinst script**: The Debian post-install script now runs in four phases — system setup, per-user config installation, audio stack restart, and post-install validation. Each phase handles errors independently: per-user failures are reported individually with the username, and the final summary reports whether the install was clean or had warnings. Previously, all 11 operations used `|| true` and the user always saw "installed successfully" regardless of what actually happened.
- **Post-install validation**: After installation, the script verifies that `motu-mk5d` and `motu-ctl` are on PATH, the ALSA card profile exists, and udev rules are in place. Any missing component is reported as a warning.
- **Makefile preflight checks**: `make install` now runs a `preflight` target first, checking for required tools (`pw-loopback`, `pw-cli`, `systemctl`) and warning about optional ones (`pactl`, `pw-metadata`). After the audio stack restart, it verifies that PipeWire and WirePlumber came back up.
- **RPM spec updated**: The Fedora RPM `%post` scriptlet now has the same phased error handling as the Debian postinst — per-user failure reporting and individual error messages instead of blanket `|| true`.

### Diagnostics and issue reporting

- **`motu-ctl diagnose` command**: Generates a complete system diagnostic report covering: package version, kernel/distro/arch, USB device presence (MOTU vendor `07fd`), CDC Ethernet network interface, PipeWire version and MOTU node listing, WirePlumber device state, ALSA node discovery, WebSocket device connectivity, running `pw-loopback` process count, systemd daemon status, and the last 30 lines of daemon logs. Output is markdown, ready to paste directly into a GitHub issue. The report ends with the issue submission URL.
- **Actionable error messages**: Every user-facing error now includes three parts: what broke, the likely cause, and what to do next. Connection failures tell the user to check USB and run `ip link`. Router failures mention checking `pw-loopback` installation. Persistent issues point to `motu-ctl diagnose`. No more bare "failed to X" messages.
- **GitHub issue templates**: Four structured forms — general bug report, audio problems (with symptom checkboxes: no sound, crackling, wrong device, etc.), connection issues (device not found, drops, crash-loops), and feature requests. Audio and connection templates require diagnostic output. Blank issues are disabled — the template chooser links to the troubleshooting guide first.

### Self-update

- **`motu-ctl update` command**: Checks GitHub releases for the latest version, compares with the installed version, and offers to download and install the update. Detects the distro from `/etc/os-release` and downloads the correct package format — `.deb` for Debian/Ubuntu/Pop!_OS, `.rpm` for Fedora/RHEL, `.pkg.tar.zst` for Arch. Installs via the native package manager (`dpkg`, `dnf`, `pacman`). Use `--check` to check for updates without installing. The post-install scripts handle the audio stack restart.

### Packaging and build infrastructure

- **Docker-based package builders**: Reproducible builds for all three distro families. Each Dockerfile creates a source tarball and runs the standard packaging toolchain (`cargo-deb`, `rpmbuild`, `makepkg`) inside the correct base image. Run `pkg/build-packages.sh` to build all three, or `pkg/build-packages.sh deb|rpm|arch` for a single format. Output goes to `target/packages/`.
- **`.dockerignore`**: Excludes `target/` and `.git/` from the Docker build context (was sending 1.3GB per build without this).

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
