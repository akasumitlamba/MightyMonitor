# Installer packages

## Debian / Ubuntu alpha package

Build native optimized binaries, then generate an installable .deb:

```sh
cargo build --release --locked -p mightymonitor-host -p mightymonitor-viewer
python3 scripts/package-linux.py --version '0.0.1~alpha.1'
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
sudo apt install ./mightymonitor_0.0.1~alpha.1_amd64.deb
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
