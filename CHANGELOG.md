# Changelog

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
