# MightyMonitor project handover

Last reviewed: 20 October 2026. This is the starting point for a new contributor
or AI assistant. Verify repository/release state before relying on this snapshot.

## Intended finished app

An original, clean-room Rust application for Windows 11, Linux and Android
(the user's Android targets are 10, 11, 14 and 16). Devices on the same local
network should share, view and optionally control screens in either direction:

- Show a phone screen in a PC window and control it with keyboard/mouse.
- Show and control a Windows/Linux screen from a phone or another PC.
- Create real extra desktop space on a source PC and view it on another device.
  A mirrored window is not an extended display.
- Host and view concurrently: eventually Linux extends to Windows while Windows
  extends to Linux, with an Android session alongside them. Control and disconnect
  must remain scoped to each session, with no recursive capture or input loops.
- Wi-Fi is primary. Reuse IP over Ethernet, Android USB networking and supported
  Thunderbolt/USB4 networking. An arbitrary USB-C cable between two host ports
  does not guarantee a network link. Never promise zero latency.
- Simple View/Share navigation, minimal text, light neutral/green styling and a
  modern flat project icon. No blue/dark default. No proprietary product branding
  in marketing copy. Proprietary code/protocols/assets must not be copied.
- Local accounts/cloud services are unnecessary. The project website must stay
  plain HTML/CSS/JS usable from disk and GitHub Pages, without Sites hosting.

`CROSS_DEVICE.md` is the detailed product specification; `DEVICE_ACCEPTANCE.md`
is the physical-device checklist and untested route matrix. Android is an
extension *viewer*; a separate Android desktop is outside the initial scope.

## Verified state

PRs #38–46 are merged upstream. #44 added remote control and concurrent desktop
viewers; #45 added experimental Wayland extension. Check newer PRs before editing.

| Capability | Implementation and evidence | Remaining acceptance/limits |
| --- | --- | --- |
| Linux mirror/view | X11 and system-authorized PipeWire/Wayland; shared H.264 | Physical two-device Wi-Fi/recovery/latency matrix |
| Windows mirror/view | GStreamer DXGI capture and H.264 decoding | Physical Windows capture/control and cross-device tests |
| Android mirror/view | MediaProjection capture, MediaCodec decode, Rust JNI networking | PC→phone and phone→desktop real video passed on Android 10/11/14/16 **emulators**; physical phones unverified |
| Secure sessions | Pinned TLS 1.3, per-share private certificate/token link, QR, fresh identity per Share | Persistent device pairing not implemented; opt-in discovery passes Rust LAN and Android emulator tests on the development branch |
| Desktop control | Windows/X11 native input; Wayland RemoteDesktop portal; focused picture forwards input | Actual X11 injection/held-key cleanup passed on isolated Xvfb; physical Windows and interactive Wayland input unverified; primary/default desktop control only |
| Android control | User-enabled Accessibility service, optional user-selected IME, local per-share and viewer grants | Actual Android button tap, text entry and revocation passed on all four emulator versions; physical phones unverified |
| Concurrent viewers | Desktop host accepts four viewers, one controller at a time | Two encrypted H.264 viewers and controller handoff passed on Xvfb; Android host supports one viewer; opposite-direction multi-device acceptance remains |
| Linux Extend | Portal Virtual source, advertised capability and returned type checked; Mirror/Extend UI | Live KDE Wayland: real extra output appeared, three H.264 frames decoded over secure loopback, normal Stop removed output and restored original active displays. Layout/window movement, virtual-display input, other compositors and physical Wi-Fi unverified |
| Windows/X11 Extend | Explicit unsupported error | Actual virtual-output/driver implementation missing |
| Installers | Linux DEB, Windows per-user setup, Android debug/test APK | Builds pass, including Windows install/runtime/uninstall; production signing, stable release publication and update continuity not established |
| Website | Static `website/dist`; latest stable GitHub installer asset lookup | Keep status current; do not confuse workflow artifacts with published releases |

The full app is **not complete**. There is no defensible measured completion
percentage. Builds, emulator tests and a local live test are distinct evidence.

## Code map

- `core/`: versioned messages/framing, generic transports, reconnect, bitrate,
  normalized input events, platform traits. See PROTOCOL.md and ABR.md.
- `discovery/`: untrusted mDNS presence, explicit public metadata whitelist, bounded
  desktop device list; Android uses platform NSD. See DISCOVERY.md for test limits.
- `security/`: TLS 1.3, certificate pinning, private session invitations and
  asynchronous/blocking handshakes. See SECURE_CONNECTIONS.md.
- `codec-gst/`: H.264 encoding/decoding and Windows capture adapter.
- `host-linux/`: X11 capture, Wayland portal/PipeWire, shared latest-frame capture,
  optional RemoteDesktop portal input and Virtual monitor support.
- `host/`: `mm-host`, capture/encoder pipeline, per-viewer sessions, native input,
  exclusive controller ownership and managed Stop lifecycle.
- `viewer/`: eframe/egui `mm-viewer`, View/Share, QR, decoder, focused input,
  child-host lifecycle. `examples/receive_smoke.rs` receives actual video.
- `android-bridge/`: original Rust phone host/client and JNI bridge.
- `android/`: native Java Activity, MediaCodec/MediaProjection foreground service,
  optional Accessibility/IME; debug-only InputTestActivity and instrumentation.
- `packaging/`, `scripts/`, `.github/workflows/`: installers and platform checks.
- `website/dist/`: deployable HTML/CSS/JS, bundled fonts and original icon.
- `docs/STATUS.md`: historical progress log, not a concise current support matrix.

Read REMOTE_CONTROL.md, EXTENDED_DISPLAYS.md, INSTALLERS.md, WINDOWS.md and
ANDROID instructions in `android/README.md` before changing their features.

## Security and lifecycle invariants

- A viewing link is a bearer secret. Do not broadcast it through discovery,
  store it in preferences, print it in diagnostics or put it in screenshots/PRs.
- TLS pin/token failures are terminal: no fallback, public trust bypass or downgrade.
  Legacy plaintext viewing is explicit opt-in and cannot enable remote control.
- Sharing is view-only by default. Control needs local host permission plus a
  separate viewer request. Android also requires real OS Accessibility consent;
  optional keyboard enable/selection stays with the user. Wayland uses OS consent.
- Disconnect, focus loss, revocation and Stop release held input. Reconnect drops
  stale input and needs a fresh control grant. Queues/frame buffers stay bounded.
- Four desktop viewers share exclusive controller ownership. A view-only peer
  must not release another viewer's keys on the shared Wayland input backend.
- Never automate real user input during tests on their active desktop. Android
  secure-setting commands in emulator instrumentation are test-only, not app code.
- Windows protected/elevated screens and Android protected content retain OS limits.

## Next work, in order of useful independent milestones

1. **Physical acceptance and usability:** install on Windows 11 and the supported
   phones; record actual Wi-Fi playback, motion latency median/p95, sustained
   sessions, rotation/scaling, focus, revocation, congestion and reconnection.
   Finish KDE virtual-monitor layout/input tests and another supported compositor.
   OS permission dialogs require the user; do independent work while waiting.
2. **Discovery and pairing UX:** opt-in desktop/Android presence discovery is
   implemented on the current development branch. A real local Rust advertise/
   resolve/remove test passed and confirmed no TCP connection. Android 10/11/14/16 NSD/video/control tests passed after bounded SDK-download
   retry; the control test now includes Hindi and a check mark. Physical cross-device discovery
   remains unverified. Continue toward smooth
   pairing without broadcasting tokens
   or treating nearby peers as trusted. Preserve explicit sharing/control consent.
   Plan certificate identity, pairing confirmation, revocation and persistence
   before replacing private links. Android discovery needs lifecycle/network tests.
3. **Windows extended display:** read WINDOWS_EXTENSION_PLAN.md first. It records
   capture/input targeting and an unsafe stale-monitor-list race in a candidate
   driver protocol; no driver dependency has been added. Implement/integrate a license-compatible virtual
   display driver and monitor/input mapping. Research driver signing/distribution.
   Do not automatically install a third-party root certificate or disable driver
   signature enforcement. Production signing/account/certificate access is an
   external dependency to explain concretely, not silently bypass.
4. **Concurrent product behavior:** explicit source selection, independent session
   identities and opposite-direction streams; prevent recursive capture/input;
   increase phone-host peers only with bounded-resource and lifecycle acceptance.
5. **Wired setup:** read-only desktop and Android IPv4 interface pickers are
   implemented in development; see NETWORK_SETUP.md. Deep links now prefill the
   desktop viewer and per-user installer handlers are implemented, with Windows install/uninstall handler validation passed. Continue to test Wi-Fi/Ethernet/USB
   networking, and test supported USB4/Thunderbolt plus Android tethering/tunnels.
   Document actual hardware support instead of assuming a Type-C cable works.
6. **Production delivery:** stable Android signing/update keys, Windows signing,
   dependency/license review, installer upgrade/uninstall tests, versioned
   published installer releases, then latest-release website download validation.
   Never commit signing credentials. Browser streaming is a later roadmap feature;
   the static website itself is not a screen viewer.

Do not leave later milestones as unexplained "pending": implement what can be
implemented, identify physical/hardware/signing dependencies precisely, and keep
this file's evidence matrix current as work lands.

## Build and meaningful checks

Use the Rust toolchain/MSRV and exact dependency versions specified in the repo
(current workspace MSRV is 1.85). Linux needs native GStreamer, PipeWire/SPA,
Clang, X11 and GUI development libraries; follow workflow dependency lists.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --features android-bridge/jni --locked -- -D warnings
MM_REQUIRE_CODECS=1 cargo test --workspace --features android-bridge/jni --locked
node website/tests/downloads.cjs
```

`MM_REQUIRE_CODECS=1` makes codec availability meaningful. Ignored live tests are
not automatic passes. Run `host/tests/x11_input.rs` only on an isolated Xvfb with
`MM_ALLOW_INPUT_SMOKE=1` and sequential tests. Current platform workflows also
check Windows, PipeWire and MSRV. Android's workflow builds three ABIs and runs
four real video/control/discovery tests on APIs 29/30/34/36. Instrumentation must grant
Accessibility **after** its process restart and wait for the specific target
editor's IME connection; prebinding or accepting any stale IME caused failures.

For Linux Extend, start `mm-host --secure --extend --managed` with a local bind,
keep its stdin open, approve the virtual monitor, receive real H.264 video, send
`STOP`, and compare active displays before/during/after. A closed stdin immediately
stops a managed host. Keep its invitation private and delete test credentials.

Installer workflows are `linux-installer.yml` and `windows-installer.yml`.
Android builds use `android-viewer.yml`. They can be dispatched on a feature ref;
workflow artifacts are temporary development downloads, not stable releases.
Do not rely on another contributor's `/tmp` dependency paths or local binaries.
Recreate dependencies from the checked-in workflows. Avoid concurrent builds from
multiple worktrees into one cargo target directory because stale output can mislead.

## Repository and PR workflow

Upstream: `reduckyLang/MightyMonitor`. User fork: `akasumitlamba/MightyMonitor`.
Check `git remote -v`, clean/dirty state and applicable AGENTS.md before working.
User authorizes commits to their fork and milestone PRs to upstream; the user or
maintainer merges them. Prefer `codex/` branch names. Do not discard local work.

The maintainer uses **squash merge**. This preserves the source but changes ancestry.
PRs #40–45 needed synchronization when based on pre-squash feature history.
Before each new PR:

1. Fetch upstream main and confirm the previous prerequisite really merged.
2. Compare its reviewed branch tree with the squash merge/main tree.
3. Synchronize the new feature branch; resolve conflicts by inspecting code.
   Taking feature versions wholesale is safe only if tree identity was proven
   and no independent upstream changes would be lost. Never assume it.
4. Verify the PR diff contains only the new milestone and is mergeable. Push
   normally; avoid forced rewrites of someone else's work.
5. Run relevant checks, include concrete evidence and limitations, open and attach
   the PR. Avoid overlapping dependent PRs. Document the next merge order clearly.

No new scheduling/hosting service is needed to keep the website fresh: update it
with each milestone. Its release buttons must point only to real published
installers from the configured upstream repository, with honest unavailable states.

## Keeping this handover useful

At each milestone update: the date, support/evidence matrix, remaining work,
new module map entries, tests and known failure causes, relevant PRs, and website
status copy. Link this guide from README and keep AGENTS.md pointing here. Keep
personal conversation history, local machine identifiers, secrets and transient
execution-session details out of the repository. This guide should explain the
project sufficiently for a contributor with no access to previous chats.

Current development note: the user has asked to keep implementing/testing without
pausing for PRs or merges, and to create PRs later. Continue on the working branch
with normal commits/checkpoints; do not treat an unopened PR as a work blocker.

The current development branch also adds desktop Keyboard text entry for Unicode/
composition, with held-input release and dialog text/undo cleanup. Its text path
uses the existing bounded Unicode protocol. The Android control acceptance test
now includes non-ASCII text. Do not claim its new test result until it completes.

Latest validated checkpoint: discovery, network selection, private-link startup
and Unicode Keyboard compile/tests passed; Windows install/runtime/uninstall plus
URI registration cleanup passed and the Linux alpha.7 installer built. New work
after that checkpoint fixes Android stale pointer clicks in rotation letterboxes,
normal Stop status, unavailable selected-network failure and remote-IME fullscreen
behavior, and keeps an active visible Android viewer awake. Regression tests are
being rerun. Check current results before upgrading these claims to acceptance.


### Active development checkpoint: Android viewing usability

The current branch adds a Full screen button to the Android viewer. It hides
connection/navigation controls while retaining an explicit Exit full screen
button over the picture, preserves aspect ratio and restores controls on exit or
disconnect. Active viewing keeps the display awake; background/disconnect clears
that request. Java main/debug/instrumentation sources compile locally. The actual
video test requires three more rendered frames both entering and leaving full
screen. Run 39373557186 passed all four tests on Android 10/11/14/16, including
full-screen transitions, rotated-pointer targeting, Unicode typing and revocation.
Android Share controls now scroll on small/landscape screens; desktop discovery
uses a compact scrolling list. Full-screen transitions release held input.
No new PR is requested: continue on `codex/device-discovery` and create one later.


Installer acceptance was expanded in the current branch to Linux actual
install/reinstall/runtime/desktop validation/purge and Windows reinstall. Linux run 39373557147 and Windows run 39373557232 passed these new lifecycle
checks at commit 27173aa. These run on
isolated CI machines; no package install or uninstall was performed on the user's
active Linux desktop.


The artifact-manifest follow-up adds SHA256SUMS and BUILD_INFO.json to all three
platform artifact bundles. Local checks passed; verify its fresh artifact jobs
before marking packaging of those records accepted. These unsigned records do
not provide production publisher signing.


Android notification follow-up: the first Share on API 33+ requests optional
POST_NOTIFICATIONS permission once, then independently requests MediaProjection
consent whether notifications were allowed or denied. App Stop remains available.
The existing emulator capture tests handle/deny the optional notification prompt;
check fresh execution before claiming this path accepted. Java compilation passes.
