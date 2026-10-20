# Installer packages

## Debian / Ubuntu alpha package

Build native optimized binaries, then generate an installable .deb:

```sh
cargo build --release --locked -p mightymonitor-host -p mightymonitor-viewer
python3 scripts/package-linux.py --version '0.0.1~alpha.7'
```

The script requires Python 3, dpkg, dpkg-deb, and dpkg-shlibdeps (dpkg-dev).
It validates the ELF architecture, derives linked-library ABI requirements using
dpkg-shlibdeps, and declares dynamically loaded GUI/codec plugins explicitly.
Packaging never installs software or executes application binaries.

The package contains the host and viewer together in /usr/lib/mightymonitor,
a mightymonitor command, an application-menu entry, the project icon, and the
project license/NOTICE. All package ownership is normalized to root:root without
requiring root during creation. It contains no service, boot autostart, privileged
input device setup, or network listener enabled by installation.

Install a package built for your distribution/architecture using its graphical
package installer, or:

```sh
sudo apt install ./mightymonitor_0.0.1~alpha.7_amd64.deb
```

Then open MightyMonitor from the application menu. apt resolves the declared
GStreamer/GUI runtime dependencies. Remove it using your package manager or
`sudo apt remove mightymonitor`.

The Build Linux installer workflow produces an artifact built on Ubuntu 24.04.
A local build records ABI requirements of the machine where it was built; it is
not automatically compatible with older distributions. Other package formats
and clean-machine installation acceptance remain pending.

## What this package is

An alpha development installer for the Linux X11/Wayland sharing and viewing app.
It is not a declaration that the complete cross-platform product is finished.
The GUI uses private viewing links over pinned TLS; see
[connection security](SECURE_CONNECTIONS.md). Remote input, independent extended
displays, and complete multi-device acceptance remain unfinished.

## Releases and downloads

The packaging workflow uploads build artifacts, not a public stable release.
After device testing, maintainers can attach the .deb to a GitHub release. The
static project website discovers installable assets from the latest stable
release; an unpublished/prerelease CI artifact does not appear as a stable download.
No source archive is substituted for an installer.

## Windows installer development milestone

The `Build Windows installer` workflow creates a per-user x64 Setup executable
using the runner's Inno Setup 6 compiler. It installs the host, viewer, launcher,
Start menu entry, optional desktop shortcut, icon and an uninstaller. The first
installation downloads the pinned official GStreamer 1.28.8 package over HTTPS,
verifies its SHA-256, and installs it under the current user's local app folder.
An internet connection and additional disk space are required. No runtime DLLs
are redistributed inside our Setup executable.

Use the Start menu shortcut: its Rust launcher configures the dedicated runtime
for the viewer and its owned host without editing the system PATH. Setup enables
no service, firewall exception, driver or network listener. The runtime is a
separately installed prerequisite and remains after app removal; its own Windows
uninstall entry can remove it when no longer needed.

The workflow exercises silent install, installed decoder initialization with the
build SDK removed from PATH, and app uninstall. An installer artifact is not a
signed stable release or proof of physical Windows 11 screen capture. Clean-device
installation, firewall approval and real Wi-Fi use still require acceptance.


Development alpha.7 adds local discovery, network selection and desktop private-link
handling. Opening a private URI prefills the viewer, with Connect still explicit.
The Windows handler is registered per user and removed on uninstall; the Linux
desktop file declares x-scheme-handler/mightymonitor. APK signing/update continuity
and production release signing are still separate unfinished work.


The next packaging checks also install and reinstall the Linux package on an
isolated CI runner, initialize the installed decoder, validate the desktop entry
and remove the app/assets. Windows setup is exercised twice before its runtime,
URI registration and uninstall checks. These checks passed at checkpoint 27173aa on Linux and Windows; same-version
reinstall is not proof of all future version upgrades.


## Build identity and file checks

New build artifacts include `SHA256SUMS` and `BUILD_INFO.json`. The latter records
the source commit, whether tracked sources changed during the build, file sizes
and SHA-256 hashes. Paths are relative to the artifact root, including Android's
APK subdirectories. On Linux, run `sha256sum -c SHA256SUMS` after extracting the
whole artifact. On Windows, `Get-FileHash <installer> -Algorithm SHA256` can be
compared with the manifest. These are unsigned integrity/provenance records, not
publisher signatures or a substitute for downloading from a trusted workflow.
The manifest generator was checked locally against independent hashes and nested
paths; fresh packaging workflow validation is still required.
