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
| Platform | Host (shares screen) | Viewer (displays screen; control pending) |
|----------|----------------------|--------------------------------|
| Linux    | implemented, desktop validation pending | implemented, desktop validation pending |
| Windows  | implemented, physical-device validation pending | implemented, physical-device validation pending |
| Android  | planned | alpha APK; playback tested on Android 10/11/14/16 emulators |
| Browser (ChromeOS, TVs, old laptops, Fire tablets) | - | planned |

## Architecture (Cargo workspace)
- `core`: protocol (serde + bincode, version byte first), framing, input events, platform traits (Capture, Encoder, Decoder, InputInjector), adaptive bitrate and reconnect logic.
- `host-linux`: X11 capture and an optional PipeWire portal backend. The current host CLI uses X11; input injection is planned.
- Windows host: GStreamer DXGI desktop capture with shared H.264/TCP streaming; input injection is planned.
- `viewer`: egui/eframe viewer for Linux and Windows.
- Browser streaming: planned; the static project website is not a streaming viewer.
- `android-bridge` / `android`: shared Rust networking via JNI, with native Java UI and MediaCodec decoding. Phone capture and remote input are planned. See [Android build/install instructions](android/README.md).
- Networking: TCP today. QUIC and mDNS discovery are planned.

The workspace includes `core`, Linux capture, GStreamer H.264 codecs, a host,
a desktop viewer, and latency tools. Linux host/viewer paths are implemented;
real desktop and Wi-Fi acceptance is still pending. See [docs/HOST.md](docs/HOST.md),
[docs/VIEWER.md](docs/VIEWER.md), [docs/WINDOWS.md](docs/WINDOWS.md), and [docs/ROADMAP.md](docs/ROADMAP.md).

The cross-device product requirements and acceptance gates are in
[docs/CROSS_DEVICE.md](docs/CROSS_DEVICE.md). Mirroring, remote control, and
extended displays are separate capabilities; the latter two are not implemented.

## Clean-room policy
Our protocol and code are original. No code, protocols, drivers, or assets are
copied from proprietary applications. Third-party open-source work may be studied
only where its license permits, with appropriate attribution in NOTICE.

## Known limits
- The Linux host CLI currently requires X11. Wayland portal integration and physical-device validation are pending.
- Android currently views PC streams only. Phone capture and control are not implemented.
- Virtual extended monitors need OS-specific drivers and stay optional.

## Status
Early development: core foundation complete; Linux streaming components exist.
Windows capture and H.264 decoding are implemented with GStreamer; physical-device
validation is pending. An Android viewer APK is built and real desktop video
decoding passed on Android 10, 11, 14 and 16 emulators; physical phone tests
remain pending. Phone capture, input forwarding, secure pairing, discovery and
virtual displays remain future work. Current streaming is unauthenticated and unencrypted; use a trusted
LAN for development. See [docs/STATUS.md](docs/STATUS.md).

## License
Apache-2.0, see [LICENSE](LICENSE) and [NOTICE](NOTICE).

## Linux alpha installer

Debian/Ubuntu package generation and local installation instructions are in
[docs/INSTALLERS.md](docs/INSTALLERS.md). This packages the Linux development
prototype; it does not mean all cross-device features are complete.
