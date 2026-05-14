PREFIX ?= /usr/local
BINDIR = $(PREFIX)/bin
UDEVDIR = /etc/udev/rules.d
WPDIR = $(HOME)/.config/wireplumber/main.lua.d
SYSTEMD_USER_DIR = $(HOME)/.config/systemd/user

.PHONY: build install uninstall clean

build:
	cargo build --release

install: build
	install -Dm755 target/release/motu-mk5d $(BINDIR)/motu-mk5d
	install -Dm755 target/release/motu-ctl $(BINDIR)/motu-ctl
	install -Dm644 install/udev/89-motu-mk5.rules $(UDEVDIR)/89-motu-mk5.rules
	install -Dm644 install/wireplumber/51-motu-mk5.lua $(WPDIR)/51-motu-mk5.lua
	install -Dm644 install/systemd/motu-mk5d.service $(SYSTEMD_USER_DIR)/motu-mk5d.service
	udevadm control --reload-rules || true
	systemctl --user daemon-reload || true
	systemctl --user enable motu-mk5d.service || true
	@echo ""
	@echo "Installed. Plug in the mk5 or run: systemctl --user start motu-mk5d"

uninstall:
	systemctl --user stop motu-mk5d.service || true
	systemctl --user disable motu-mk5d.service || true
	rm -f $(BINDIR)/motu-mk5d $(BINDIR)/motu-ctl
	rm -f $(UDEVDIR)/89-motu-mk5.rules
	rm -f $(WPDIR)/51-motu-mk5.lua
	rm -f $(SYSTEMD_USER_DIR)/motu-mk5d.service
	udevadm control --reload-rules || true
	systemctl --user daemon-reload || true
	@echo "Uninstalled."

clean:
	cargo clean
