# MightyMonitor

MightyMonitor is a free, open-source application being built to share, control,
and extend screens across Linux, Windows, and Android devices over a local
network. View another device in a window, use your keyboard and mouse or touch
to control it, and turn another device into an additional display as these
capabilities become available.

Built in Rust, with local connections and no cloud accounts. Wi-Fi is the primary
connection method; wired networking is part of the planned experience. macOS and
iOS are not currently targeted.

## Goals
- Share a screen between Linux, Windows and Android (host to viewer, any direction).
- View an Android phone on a PC and control it from the PC.
- Stream a PC screen (or later a virtual extended monitor) to a phone or other device.
- **Fully wireless**: everything works over Wi-Fi/LAN with no cable, including Android (wireless adb, Wi-Fi streaming), with adaptive bitrate and automatic reconnects.
- Automatic device discovery on the LAN (mDNS).

## Platform matrix
| Platform | Host (shares screen) | Viewer (displays and controls) |
|----------|----------------------|--------------------------------|
| Linux    | implemented, desktop validation pending | implemented, desktop validation pending |
| Windows  | planned | GUI exists; real decoder planned |
| Android  | planned | planned |
| Browser (ChromeOS, TVs, old laptops, Fire tablets) | - | planned |

## Architecture (Cargo workspace)
- `core`: protocol (serde + bincode, version byte first), framing, input events, platform traits (Capture, Encoder, Decoder, InputInjector), adaptive bitrate and reconnect logic.
- `host-linux`: PipeWire (ashpd) with X11 fallback, GStreamer H.264, uinput.
- `host-windows`: Windows.Graphics.Capture / DXGI, H.264, SendInput.
- `viewer-desktop`: egui/eframe viewer for Linux and Windows.
- `web-viewer`: static page served by the host (WebRTC/WebCodecs).
- `android`: Rust library (cargo-ndk, uniffi) plus a thin Kotlin app (MediaProjection, MediaCodec, Accessibility or wireless adb for input).
- Networking: tokio TCP first, QUIC (quinn) later. Discovery: mdns-sd.

The workspace includes `core`, Linux capture, GStreamer H.264 codecs, a host,
a desktop viewer, and latency tools. Linux host/viewer paths are implemented;
real desktop and Wi-Fi acceptance is still pending. See [docs/HOST.md](docs/HOST.md),
[docs/VIEWER.md](docs/VIEWER.md), and [docs/ROADMAP.md](docs/ROADMAP.md).

The cross-device product requirements and acceptance gates are in
[docs/CROSS_DEVICE.md](docs/CROSS_DEVICE.md). Mirroring, remote control, and
extended displays are separate capabilities; the latter two are not implemented.

## Clean-room policy
Our protocol and code are original. No code, protocols, drivers, or assets are
copied from proprietary applications. Third-party open-source work may be studied
only where its license permits, with appropriate attribution in NOTICE.

## Known limits
- Wayland requires a permission prompt per capture session.
- Android sharing needs one-time user approval; remote control needs the Accessibility service or (wireless) adb.
- Virtual extended monitors need OS-specific drivers and stay optional.

## Status
Early development: core foundation complete; Linux streaming components exist.
Windows capture/decoding, Android, input forwarding, and virtual displays remain
future work. Current streaming is unauthenticated and unencrypted; use a trusted
LAN for development. See [docs/STATUS.md](docs/STATUS.md).

## License
Apache-2.0, see [LICENSE](LICENSE) and [NOTICE](NOTICE).

## Linux alpha installer

Debian/Ubuntu package generation and local installation instructions are in
[docs/INSTALLERS.md](docs/INSTALLERS.md). This packages the Linux development
prototype; it does not mean all cross-device features are complete.
