PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
UDEVDIR = /etc/udev/rules.d
ACPDIR = /usr/share/alsa-card-profile/mixer/profile-sets
WPDIR = $(HOME)/.config/wireplumber/main.lua.d
SYSTEMD_USER_DIR = $(HOME)/.config/systemd/user
PULSE_CONFDIR = $(HOME)/.config/pipewire/pipewire-pulse.conf.d

.PHONY: build install uninstall clean preflight

build:
	cargo build --release

preflight:
	@echo "Checking dependencies..."
	@command -v pw-loopback >/dev/null 2>&1 || { echo "ERROR: pw-loopback not found (install pipewire)"; exit 1; }
	@command -v pw-cli >/dev/null 2>&1 || { echo "ERROR: pw-cli not found (install pipewire)"; exit 1; }
	@command -v systemctl >/dev/null 2>&1 || { echo "ERROR: systemctl not found"; exit 1; }
	@command -v pactl >/dev/null 2>&1 && echo "  pactl: ok" || echo "  WARNING: pactl not found — volume enforcement will not work"
	@command -v pw-metadata >/dev/null 2>&1 && echo "  pw-metadata: ok" || echo "  WARNING: pw-metadata not found — sample rate enforcement will not work"
	@echo "  pw-loopback: ok"
	@echo "  pw-cli: ok"
	@echo "Dependencies OK."

install: build preflight
	install -Dm755 target/release/motu-mk5d $(BINDIR)/motu-mk5d
	install -Dm755 target/release/motu-ctl $(BINDIR)/motu-ctl
	install -Dm644 install/alsa-card-profile/motu-ultralite-mk5.conf $(ACPDIR)/motu-ultralite-mk5.conf
	install -Dm644 install/udev/89-motu-mk5.rules $(UDEVDIR)/89-motu-mk5.rules
	install -Dm644 install/wireplumber/51-motu-mk5.lua $(WPDIR)/51-motu-mk5.lua
	install -Dm644 install/systemd/motu-mk5d.service $(SYSTEMD_USER_DIR)/motu-mk5d.service
	install -Dm644 install/systemd/wireplumber-motu-mk5.conf $(SYSTEMD_USER_DIR)/wireplumber.service.d/motu-mk5.conf
	install -Dm644 install/pipewire-pulse/50-motu-wine-routing.conf $(PULSE_CONFDIR)/50-motu-wine-routing.conf
	udevadm control --reload-rules || true
	systemctl --user stop pipewire.socket pipewire-pulse.socket pipewire pipewire-pulse wireplumber 2>/dev/null || true
	pkill -u $(USER) -x pipewire 2>/dev/null || true
	pkill -u $(USER) -x wireplumber 2>/dev/null || true
	pkill -u $(USER) -x pipewire-pulse 2>/dev/null || true
	sed -i '/alsa_card.usb-MOTU_UltraLite/d' $(HOME)/.local/state/wireplumber/default-profile 2>/dev/null || true
	sleep 1
	systemctl --user daemon-reload || true
	systemctl --user enable motu-mk5d.service || true
	systemctl --user start pipewire.socket pipewire-pulse.socket wireplumber
	@sleep 2
	@systemctl --user is-active pipewire.service >/dev/null 2>&1 && echo "PipeWire: running" || echo "WARNING: PipeWire did not restart — run 'systemctl --user start pipewire.socket pipewire-pulse.socket wireplumber' manually"
	@systemctl --user is-active wireplumber.service >/dev/null 2>&1 && echo "WirePlumber: running" || echo "WARNING: WirePlumber did not restart"
	@echo ""
	@echo "Installed. Audio stack restarted with new profiles."

uninstall:
	systemctl --user stop motu-mk5d.service || true
	systemctl --user disable motu-mk5d.service || true
	rm -f $(BINDIR)/motu-mk5d $(BINDIR)/motu-ctl
	rm -f $(ACPDIR)/motu-ultralite-mk5.conf
	rm -f $(UDEVDIR)/89-motu-mk5.rules
	rm -f $(WPDIR)/51-motu-mk5.lua
	rm -f $(SYSTEMD_USER_DIR)/motu-mk5d.service
	rm -rf $(SYSTEMD_USER_DIR)/wireplumber.service.d/motu-mk5.conf
	rm -f $(PULSE_CONFDIR)/50-motu-wine-routing.conf
	udevadm control --reload-rules || true
	systemctl --user daemon-reload || true
	@echo "Uninstalled."

clean:
	cargo clean
