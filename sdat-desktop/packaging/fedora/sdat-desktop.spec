# sdat-desktop.spec — Fedora/COPR packaging for the SDAT desktop app
# (formerly tonearm-rs, the Rust port of Tonearm), https://github.com/Nichokas/SDAT
#
# Sources:
#   Source0: GitHub tag tarball of the SDAT monorepo (desktop-v%%{version}).
#   Source1: `cargo vendor` archive for the same tag, produced by
#            sdat-desktop/packaging/fedora/make-vendor.sh and attached to GitHub
#            release desktop-v%%{version}. The RPM build runs fully offline.
#
# Replaces the old tonearm-rs package; still conflicts with nichokas/tonearm (Go),
# which ships the legacy dev.dergs.Tonearm schema we also install for migration.

%global appid com.nichokas.SDAT.Desktop
%global legacy_appid dev.dergs.Tonearm

Name:           sdat-desktop
Version:        1.2.2
Release:        1%{?dist}
Summary:        Native GTK4/libadwaita TIDAL client (Rust port)

License:        GPL-3.0-or-later
URL:            https://github.com/Nichokas/SDAT
Source0:        %{url}/archive/desktop-v%{version}/SDAT-desktop-v%{version}.tar.gz
Source1:        %{url}/releases/download/desktop-v%{version}/%{name}-%{version}-vendor.tar.xz

ExclusiveArch:  x86_64 aarch64

BuildRequires:  cargo >= 1.85
BuildRequires:  rust >= 1.85
BuildRequires:  gcc
BuildRequires:  pkgconfig(gtk4) >= 4.20
BuildRequires:  pkgconfig(libadwaita-1) >= 1.8
BuildRequires:  pkgconfig(gstreamer-1.0)
BuildRequires:  pkgconfig(gstreamer-audio-1.0)
BuildRequires:  pkgconfig(gstreamer-base-1.0)
BuildRequires:  pkgconfig(libsecret-1)
BuildRequires:  pkgconfig(openssl)
BuildRequires:  pkgconfig(dbus-1)
BuildRequires:  glib2-devel
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       hicolor-icon-theme
# Playback (HTTPS streaming, decoding, DASH/HLS)
Requires:       gstreamer1-plugins-base%{?_isa}
Requires:       gstreamer1-plugins-good%{?_isa}
Requires:       gstreamer1-plugins-bad-free%{?_isa}
Requires:       glib-networking%{?_isa}
Recommends:     (gnome-keyring or kwallet or keepassxc)
Suggests:       gstreamer1-libav

Conflicts:      tonearm
Provides:       tonearm-app = %{version}-%{release}
Obsoletes:      tonearm-rs < 1.2.0-1
Provides:       tonearm-rs = %{version}-%{release}

%description
SDAT is a native GTK4/libadwaita client for the TIDAL music streaming
service. This is the Rust port (gtk4-rs, libadwaita-rs, gstreamer-rs) of the
original Go application, with the same interface.

%prep
%autosetup -n SDAT-desktop-v%{version}
tar -xf %{SOURCE1}
mkdir -p .cargo
cat > .cargo/config.toml <<'CFG'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
CFG

%build
export TONEARM_VERSION=%{version}
export TONEARM_COMMIT=v%{version}
export RUSTFLAGS="%{?build_rustflags}"
cargo build --release --offline --frozen -p sdat-desktop

%install
install -Dm0755 target/release/sdat-desktop %{buildroot}%{_bindir}/sdat-desktop
install -Dm0644 sdat-desktop/internal/icons/hicolor/scalable/apps/%{appid}.svg \
    %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/%{appid}.svg
install -Dm0644 sdat-desktop/internal/icons/hicolor/symbolic/apps/%{appid}-symbolic.svg \
    %{buildroot}%{_datadir}/icons/hicolor/symbolic/apps/%{appid}-symbolic.svg
install -Dm0644 sdat-desktop/internal/icons/hicolor/128x128/apps/%{appid}.png \
    %{buildroot}%{_datadir}/icons/hicolor/128x128/apps/%{appid}.png
install -Dm0644 sdat-desktop/build/%{appid}.desktop %{buildroot}%{_datadir}/applications/%{appid}.desktop
install -Dm0644 sdat-desktop/build/%{appid}.metainfo.xml %{buildroot}%{_metainfodir}/%{appid}.metainfo.xml
install -Dm0644 sdat-desktop/internal/settings/%{appid}.gschema.xml \
    %{buildroot}%{_datadir}/glib-2.0/schemas/%{appid}.gschema.xml
# Legacy schema, read once by the settings migration from Tonearm.
install -Dm0644 sdat-desktop/internal/settings/%{legacy_appid}.gschema.xml \
    %{buildroot}%{_datadir}/glib-2.0/schemas/%{legacy_appid}.gschema.xml

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/%{appid}.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/%{appid}.metainfo.xml || :
glib-compile-schemas --strict --dry-run %{buildroot}%{_datadir}/glib-2.0/schemas

%files
%license sdat-desktop/LICENSE
%doc sdat-desktop/README.md
%{_bindir}/sdat-desktop
%{_datadir}/applications/%{appid}.desktop
%{_metainfodir}/%{appid}.metainfo.xml
%{_datadir}/glib-2.0/schemas/%{appid}.gschema.xml
%{_datadir}/glib-2.0/schemas/%{legacy_appid}.gschema.xml
%{_datadir}/icons/hicolor/scalable/apps/%{appid}.svg
%{_datadir}/icons/hicolor/symbolic/apps/%{appid}-symbolic.svg
%{_datadir}/icons/hicolor/128x128/apps/%{appid}.png

%changelog
* Mon Nov 02 2026 Nicolas Rodriguez-Alvarez <me@nichokas.eu> - 1.2.2-1
- Listen Together: guests can control playback when the host allows it
- Add Play Next to queue menus
- Listen Together sync no longer changes the playback speed
- More stable guest playback start in Listen Together

* Mon Nov 02 2026 Nicolas Rodriguez-Alvarez <me@nichokas.eu> - 1.2.0-1
- Rename to sdat-desktop, build from the SDAT monorepo, migrate Tonearm settings

* Sat Oct 31 2026 tonearm-rs CI <noreply@github.com> - 1.1.0-1
- Update to 1.1.0

* Sat Oct 31 2026 Nicolas Rodriguez-Alvarez <me@nichokas.eu> - 1.0.0-1
- Initial package of the Rust port, 1.0.0
