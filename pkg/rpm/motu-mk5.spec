Name:           motu-mk5
Version:        0.1.0
Release:        1%{?dist}
Summary:        Native Linux integration for the MOTU UltraLite mk5
License:        GPL-2.0-or-later
URL:            https://github.com/MarkWind85/motu-mk5-linux
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  alsa-lib-devel
Requires:       pipewire
Requires:       wireplumber
Requires:       alsa-lib

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

%post
udevadm control --reload-rules 2>/dev/null || true
udevadm trigger --subsystem-match=sound 2>/dev/null || true

%postun
udevadm control --reload-rules 2>/dev/null || true

%files
%license LICENSE
%doc README.md
%{_bindir}/motu-mk5d
%{_bindir}/motu-ctl
%{_datadir}/alsa-card-profile/mixer/profile-sets/motu-ultralite-mk5.conf
%{_datadir}/wireplumber/main.lua.d/51-motu-mk5.lua
%config(noreplace) %{_sysconfdir}/udev/rules.d/89-motu-mk5.rules
%{_prefix}/lib/systemd/user/motu-mk5d.service
