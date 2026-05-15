PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
UDEVDIR = /etc/udev/rules.d
ACPDIR = /usr/share/alsa-card-profile/mixer/profile-sets
WPDIR = $(HOME)/.config/wireplumber/main.lua.d
SYSTEMD_USER_DIR = $(HOME)/.config/systemd/user
PULSE_CONFDIR = $(HOME)/.config/pipewire/pipewire-pulse.conf.d

.PHONY: build install uninstall clean

build:
	cargo build --release

install: build
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
