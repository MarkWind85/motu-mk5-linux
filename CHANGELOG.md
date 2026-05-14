# Changelog

## 0.2.0

### Independent I/O profile selection

Replaces the single-profile-at-a-time model with a unified `all-io` profile that exposes all physical I/O pairs as independent PipeWire nodes. Output and input devices can now be selected independently in GNOME Settings (or any PulseAudio/PipeWire-aware app) — no more combined "Main 1/2 + Mic/Line 1/2" profiles or losing input when switching output.

**Outputs** (selectable independently):
- Main 1/2, Line 3/4, Line 5/6, Line 7/8, Line 9/10, Phones, S/PDIF Out

**Inputs** (selectable independently):
- Mic/Line 1/2, Line In 3/4, Line In 5/6, Line In 7/8, S/PDIF In

### Reliable updates

- Install and upgrade now fully restart the audio stack (including socket teardown) so new profiles take effect immediately — no reboot or manual restart required
- WirePlumber systemd drop-in clears stale MOTU profile cache on every WirePlumber start, preventing the device from getting stuck on "off"
- Applies to all install paths: deb, RPM, Arch, and Makefile

### Other changes

- WirePlumber rules now assign distinct names and priorities to each I/O node
- Uninstall (deb, RPM) cleans up the WirePlumber drop-in and systemd overrides

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
