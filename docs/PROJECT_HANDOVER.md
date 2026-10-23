# MightyMonitor project handover

Last reviewed: 23 October 2026. This is the starting point for a new contributor
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
| Desktop control | Windows/X11 native input; Wayland RemoteDesktop portal; focused picture forwards input | Actual X11 injection/held-key cleanup passed on isolated Xvfb; physical Windows and interactive Wayland input unverified; Windows explicit-screen mapping is implemented in development |
| Android control | User-enabled Accessibility service, optional user-selected IME, local per-share and viewer grants | Actual Android button tap, text entry and revocation passed on all four emulator versions; physical phones unverified |
| Concurrent viewers | Desktop host accepts four viewers, one controller at a time | Two encrypted H.264 viewers and controller handoff passed on Xvfb; Android host supports one viewer; opposite-direction multi-device acceptance remains |
| Linux Extend | Portal Virtual source, advertised capability and returned type checked; Mirror/Extend UI | Live KDE Wayland: real extra output appeared, three H.264 frames decoded over secure loopback, normal Stop removed output and restored original active displays. Layout/window movement, virtual-display input, other compositors and physical Wi-Fi unverified |
| Windows/X11 Extend | Explicit unsupported error | Actual virtual-output/driver implementation missing |
| Installers | Linux DEB, Windows per-user setup, Android debug/test APK | Builds pass, including Windows install/runtime/uninstall; Android release-signing build path verified with a disposable key; production keys, stable release publication and update continuity not established |
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
- `display-windows/`: small Win32 display snapshot/identity and explicit-pointer
  adapter; owns documented unsafe enumeration/SendInput boundaries, with no driver
  installation or display configuration changes.
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

## Current development branch and evidence

Continue on `codex/device-discovery`. The user has asked for implementation and
testing to continue without pausing for PRs or merges; create upstream PRs later.
Commit normal checkpoints to the fork. No new PR is open for these changes.

- **Verified at 27173aa:** opt-in discovery, network selection, desktop private-link
  prefill and Unicode Keyboard entry; Android full-screen viewing, keep-awake,
  scrollable Share controls, rotated-pointer/letterbox rejection, Unicode entry,
  control revocation and Stop status. Android run 39373557186 passed all four tests
  on API 29/30/34/36. CI/MSRV/PipeWire passed in 39373557265.
- **Desktop installer lifecycle:** Linux actual install/reinstall, installed
  decoder, desktop validation and purge passed in 39373557147. Windows reinstall,
  private-runtime initialization, URI registration and uninstall passed in
  39373557232. These ran on isolated CI machines, not the user's active desktop.
- **Build identity:** SHA256SUMS and BUILD_INFO.json are included with installer/
  APK artifacts. The records contain relative paths, source commit, tracked-source
  modification flag, sizes and hashes. Linux 39377608049 and Windows 39377608033
  passed the manifest follow-up; APK manifest generation passed in 39377927435.
  They are unsigned provenance/integrity records, not publisher signatures.
- **Android release build:** RELEASE_SIGNING.md documents the four private
  MM_ANDROID signing environment variables. Missing credentials fail release
  assembly. The APK job in 39377927435 passed both that refusal and actual signing/
  apksigner verification using a disposable random CI key. It deleted the key and
  signed test APK. A production key, stable update identity and publication are
  still required; the downloadable CI APK remains a debug/test build.
- **Android notification follow-up:** a first Share on API 33+ requests optional
  POST_NOTIFICATIONS once, then separately requests capture consent even after
  denial. App Stop remains available. Java compilation passes; all four emulator tests passed on API 29/30/34/36 in
  39377927435, including denial on the newer Android versions.
- **Desktop full-screen follow-up:** hide navigation/connection/status panels,
  retain an explicit exit, release held input on transitions and consume local
  Escape before remote translation. Local input regression tests and warning-free
  workspace Clippy pass. Native Linux/Windows/MSRV/PipeWire checks passed in 39378671820; physical
  full-screen UI acceptance remains unverified.

Android run 39378671813 passed actual JNI ELF and debug/release APK 16 KiB
alignment, release-signing checks, and all four video/control/discovery emulator
tests on API 29/30/34/36. Linux package install/reinstall/runtime/purge passed in
39378671812; Windows setup/reinstall/runtime/URI/uninstall passed in 39378671793.
The matching e468edb installer artifacts were downloaded and checksum-verified
locally. They remain development builds, not a published stable release.

The desktop reconnect-status follow-up passed native CI and both installers at
9fe1716. Its repeated Android suite failed the initial control tap on API 29 and
editor focus after rotation on API 36; API 30/34 passed. The readiness follow-up described below passed its confirming run. Input tests now wait for stable focused target bounds before each single gesture.
Viewer rotation separately checks visible viewport orientation/aspect and new
frames; image rendering does not require keyboard focus. Full-screen entry hides
the local keyboard. Java compilation and confirming Android run 39395701699 passed.
The control test now runs three consented Share/control/Stop sessions in one app
process. The first failure still fails the test. Avoid repeatedly force-stopping
the process merely to repeat gesture checks: that hit Android 16 Accessibility
rebind suppression before the second repeat could begin. Forced-process recovery
remains a separate, unverified lifecycle scenario.
The native Linux idle UI snapshot rendered successfully without sharing/input;
the unused Sharing off bar is removed while preserving active/error
indicators. Desktop CI and both installer lifecycle jobs passed at 734aea2.
full-screen physical-device UI acceptance is still unverified.
Android workflow filters now cover headless interop/capture dependencies and
leave GUI-only verification to desktop CI.

The support table above remains the product-level limit: no physical Windows/
phone acceptance, no Windows/X11 true extension, no persistent pairing and no
published production release. Read DEVICE_ACCEPTANCE.md for the untested routes.


**Repeated-session confirmation at bc3210e:** Android run 39395701699 passed
all four tests on Android 10/11/14/16. The viewer test required fresh video after
landscape/portrait changes. The control test completed three independently
consented share/tap/rotate/type/revoke/stop sessions in one process on each version
(12 sessions total). CI 39395701831 passed Linux, Windows, MSRV and PipeWire.
This resolves the observed test readiness failures for this run; forced-process
Accessibility recovery and physical-device reliability remain unverified.

**Windows screen selection follow-up:** `display-windows/` is an original, small
read-only Win32 adapter with the explicit unsafe FFI boundary. Share offers an
existing-screen picker and refresh. `mm-host --list-displays` lists GDI device
names; `--display DEVICE` resolves a snapshot to its exact HMONITOR for GStreamer.
The snapshot is checked before startup and each frame; disappearance, handle,
geometry or primary-status changes stop that capture instead of falling back.
At that checkpoint explicit selection was view-only; the later Windows input
follow-up below supersedes that restriction. No virtual monitor is created. Local snapshot rejection
tests and Windows adapter cross-Clippy passed; native Windows/Linux, MSRV and PipeWire checks passed at 893a490 in
run 39399148245. Linux installer/guide checks passed in 39399148203. Physical
multi-monitor/hotplug acceptance remains unverified.


The user-facing GETTING_STARTED.md guide is linked from README and included in
desktop installers. Keep its simple setup/permission/troubleshooting instructions
consistent with actual capabilities. Installer jobs check its presence and removal.


The desktop Share page now scrolls vertically so QR/settings/control actions stay
reachable in short windows. Local viewer Clippy passes with warnings denied;
native Windows/Linux/MSRV/PipeWire CI 39399759531 and both installer jobs
39399759511/39399759517 passed at 3676811. The View canvas
is unchanged.


**Later Android repeat at 53594c1:** run 39398842473 passed API 30/34/36 but
API 29 failed its initial tap in the third control session (the first two sessions
completed). Do not treat the earlier all-green run as proof of settled reliability.
The failure did not include dispatch/cancellation evidence, so its cause is not
yet established. A follow-up adds fixed-size, content-free service counters and
debug fixture DOWN/UP/CANCEL counts to test diagnostics, plus an active accessibility
window-ID readiness check. No coordinates, text, keys, tokens or viewing links are
recorded by the service. It does not retry failed gestures or alter permission
requirements; its confirming execution is pending. Physical-device testing is
deferred while implementation/build work continues.


**Desktop local revocation follow-up:** Share now offers Stop control while
locally allowed sharing is active. The private managed-host input pipe accepts
bounded REVOKE_CONTROL/STOP commands. A monotonic owner/consent signal revokes
all current input leases and prevents future grants for that share lifetime;
closed ownership fails closed. Session reads/writes remain pinned during release,
late valid input is discarded, and video continues. Failure to deliver the GUI
command ends sharing. Six real framed-socket tests with synthetic capture/input
pass, including release during a deliberately stalled video write, multi-session
revocation and later reconnect refusal. Native viewer/host Clippy passes locally;
new platform/installer runs are required. These tests do not inject OS input.

**Android diagnostic run f5afd04:** 39473898336 built the three-ABI APK and passed
API 29/30. API 34 first failed a stale notification-permission UI handle, leaving
a dialog that blocked later fixture launches. The permission helper now resolves
and clicks within one bounded loop and re-resolves stale handles before any click
was dispatched. API 36 failed the valid rotated click in session three after
three successful initial taps; its cause is not yet proven. The rotated assertion
now includes the same bounded dispatch/target counters. Main/test Java compile.
No failed remote gesture is retried. Full device testing remains deferred while
implementation and installer work continue.


**Windows selected-screen input follow-up:** main/default and explicit screens
are pinned at host start. Optional control maps the negotiated picture into the
selected screen's physical rectangle, then into absolute virtual-desktop input;
negative monitor origins and the codec's cropped odd edge are accounted for.
All input events revalidate the selected snapshot except ReleaseAll, which must
always be allowed to clean up held input. Legacy monitor indices remain view-only.
The UI retains two explicit control grants and view-only defaults. Three snapshot/
geometry tests and Windows-adapter cross-Clippy pass without injecting input;
native Windows and physical mixed-DPI/hotplug acceptance are still required.
Windows virtual-monitor creation remains unimplemented.


**23 October build continuation:** Windows selected-screen input adapter tests
and Windows-target adapter Clippy pass at 20011b1; native platform builds are
running. The previous revocation milestone 72a8299 passed CI and both desktop
installers (39476317038/39476317077/39476317117). Android APK assembly passed,
but API 30 control failed with target DOWN=1/UP=0 and one cancelled continued
gesture (39476317032). This is concrete cancellation evidence, not a permission
failure. Quick presses now allow 32 ms for an arriving release so a tap can be
submitted as one complete gesture; a held press starts a continued stroke after
that interval. Revocation clears scheduled presses. Existing tests now require
four additional independent quick taps per session without retrying failures.
Java app compilation passes; emulator confirmation is required before claiming
the cancellation issue resolved. Physical tests remain deferred by request.


2026-10-23 follow-up: Windows checks failed before compilation because the
vendor SDK response repeatedly ended prematurely, including bounded PowerShell
retries. Download now uses the runner's curl HTTP/1.1 with timeouts/retries;
pinned checksum verification is unchanged. Android run 39951349198 showed API 29
cancelling one complete 16 ms tap before target DOWN (other APIs passed). Complete
taps now use Android's tap-timeout duration; held drag segments remain short.
This is a timing hypothesis requiring confirmation, not a resolved reliability
claim. Main Java compiles locally. No permission bypass or gesture retry added.


**Virtual-output ownership foundation (23 October):** `display-windows::owned`
defines an atomic-create backend contract, exact OS adapter/target identity,
bounded mode validation and an RAII owner. It arms rollback before validation,
removes only its own opaque lease, retains ownership after failed cleanup for
retry, and documents mandatory driver-side disconnect/crash cleanup and native
operation deadlines. Four synthetic-backend tests pass alongside three existing
geometry/snapshot tests; Windows-target Clippy passes. The production backend is
not connected and no driver was added, installed or signed. Do not enable the
Windows Extend UI based on these mock tests. Next: implement the dedicated atomic
lease driver adapter and its OS identity resolution; never wrap the legacy global
Notify list as an atomic create operation.


2026-10-23: Added the native read-only resolve_output adapter for a future
owned-driver backend. It matches the exact OS adapter LUID/target ID to one active
source, rejects cloned/ambiguous/inactive paths, obtains its GDI source name, and
resolves the existing HMONITOR snapshot. Native arrays are bounded; topology
buffer growth retries at most three times. Nine synthetic identity/lifecycle/
geometry tests pass, with warning-denied local and Windows-target adapter Clippy.
This adapter is not wired to a driver yet and creates no monitor. Native runtime,
hotplug races and physical acceptance still require Windows testing.


2026-10-23 drag acceptance follow-up: native Windows/Linux CI at 1a65e8b
(39957445665) passed, and Windows installer creation/install/private-runtime/
uninstall passed (39957445548); Linux installer passed (39957445566). Android
9dce0b1 APK and API 30/34/36 emulator jobs passed (39957065889), while API 29
remained in dependency installation when checked. Do not call the timing change
fully confirmed. The test now sends a single held drag through JNI/TLS/Rust host
to a debug-only Android touch target in each of three control sessions, requiring
one DOWN, movement across the target, one UP and no CANCEL. It does not retry
failed actions. Main/debug/test Java compile locally; emulator execution of this
new drag assertion is pending. The dependency-install step now has a six-minute
limit and bounded APT transfer timeouts/retries to avoid consuming the whole
emulator job before any test runs. Physical device tests remain deferred.


2026-10-23 Android held-key cleanup: the optional IME now tracks at most 64
keys it delivered, with their original input connection and DOWN time. ReleaseAll,
local control revocation, sharing stop and IME input finish/destroy send best-effort
UP to the original connection even after the grant is false. A held key is never
transferred to a new editor. Tracking clears before cleanup delivery; disconnected
connections cannot prevent the remaining releases. The repeated encrypted-control
instrumentation now holds Right in the actual editor, revokes locally, and requires
its matching UP. Main/debug/instrumentation Java compile locally; emulator and
physical execution of this new assertion are pending. Physical testing remains
deferred by the user. Windows virtual-display creation/signing remains missing.
