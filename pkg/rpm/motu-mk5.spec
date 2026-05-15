Name:           motu-mk5
Version:        0.2.0
Release:        1%{?dist}
Summary:        Native Linux integration for the MOTU UltraLite mk5
License:        GPL-2.0-or-later
URL:            https://github.com/MarkWind85/motu-mk5-linux
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
Requires:       pipewire
Requires:       wireplumber

%description
Native Linux integration for the MOTU UltraLite mk5 audio interface.
Provides ALSA card profiles, WirePlumber integration, udev detection,
and control tools for the mk5.

%prep
%autosetup

%build
cargo build --release

%install
install -Dm755 target/release/motu-mk5d %{buildroot}%{_bindir}/motu-mk5d
install -Dm755 target/release/motu-ctl %{buildroot}%{_bindir}/motu-ctl
install -Dm644 install/alsa-card-profile/motu-ultralite-mk5.conf %{buildroot}%{_datadir}/alsa-card-profile/mixer/profile-sets/motu-ultralite-mk5.conf
install -Dm644 install/wireplumber/51-motu-mk5.lua %{buildroot}%{_datadir}/wireplumber/main.lua.d/51-motu-mk5.lua
install -Dm644 install/udev/89-motu-mk5.rules %{buildroot}%{_sysconfdir}/udev/rules.d/89-motu-mk5.rules
install -Dm644 install/systemd/motu-mk5d.service %{buildroot}%{_prefix}/lib/systemd/user/motu-mk5d.service
install -Dm644 install/systemd/wireplumber-motu-mk5.conf %{buildroot}%{_datadir}/motu-mk5/wireplumber-motu-mk5.conf
install -Dm644 install/pipewire-pulse/50-motu-wine-routing.conf %{buildroot}%{_datadir}/motu-mk5/50-motu-wine-routing.conf

%post
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger --subsystem-match=sound 2>/dev/null || true
for d in /home/*; do
    [ -d "$d" ] || continue
    install -Dm644 %{_datadir}/motu-mk5/wireplumber-motu-mk5.conf \
        "$d/.config/systemd/user/wireplumber.service.d/motu-mk5.conf" 2>/dev/null || true
    install -Dm644 %{_datadir}/motu-mk5/50-motu-wine-routing.conf \
        "$d/.config/pipewire/pipewire-pulse.conf.d/50-motu-wine-routing.conf" 2>/dev/null || true
    sed -i '/alsa_card.usb-MOTU_UltraLite/d' "$d/.local/state/wireplumber/default-profile" 2>/dev/null || true
done
loginctl list-users --no-legend 2>/dev/null | awk '{print $1}' | while read -r uid; do
    [ -n "$uid" ] || continue
    su_user=$(getent passwd "$uid" | cut -d: -f1)
    [ -n "$su_user" ] || continue
    su -l "$su_user" -c "XDG_RUNTIME_DIR=/run/user/$uid systemctl --user stop pipewire.socket pipewire-pulse.socket pipewire pipewire-pulse wireplumber" 2>/dev/null || true
    pkill -u "$uid" -x pipewire 2>/dev/null || true
    pkill -u "$uid" -x wireplumber 2>/dev/null || true
    pkill -u "$uid" -x pipewire-pulse 2>/dev/null || true
    sleep 1
    su -l "$su_user" -c "XDG_RUNTIME_DIR=/run/user/$uid systemctl --user daemon-reload" 2>/dev/null || true
    su -l "$su_user" -c "XDG_RUNTIME_DIR=/run/user/$uid systemctl --user start pipewire.socket pipewire-pulse.socket wireplumber" 2>/dev/null || true
done

%postun
udevadm control --reload-rules 2>/dev/null || true
if [ "$1" -eq 0 ]; then
    for d in /home/*; do
        [ -d "$d" ] || continue
        rm -f "$d/.config/systemd/user/wireplumber.service.d/motu-mk5.conf" 2>/dev/null || true
        rmdir "$d/.config/systemd/user/wireplumber.service.d" 2>/dev/null || true
        rm -f "$d/.config/pipewire/pipewire-pulse.conf.d/50-motu-wine-routing.conf" 2>/dev/null || true
        rmdir "$d/.config/pipewire/pipewire-pulse.conf.d" 2>/dev/null || true
    done
fi

%files
%license LICENSE
%doc README.md
%{_bindir}/motu-mk5d
%{_bindir}/motu-ctl
%{_datadir}/alsa-card-profile/mixer/profile-sets/motu-ultralite-mk5.conf
%{_datadir}/wireplumber/main.lua.d/51-motu-mk5.lua
%{_datadir}/motu-mk5/wireplumber-motu-mk5.conf
%{_datadir}/motu-mk5/50-motu-wine-routing.conf
%config(noreplace) %{_sysconfdir}/udev/rules.d/89-motu-mk5.rules
%{_prefix}/lib/systemd/user/motu-mk5d.service
