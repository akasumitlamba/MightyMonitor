# Status

2026-10-18: repository bootstrapped (workspace, `core` skeleton, CI, docs, roadmap, issues). No milestone complete yet.

2026-10-18: M1-2 stream framing + FrameDecoder + docs/PROTOCOL.md implemented on branch feat/m1-2-framing.

2026-10-18: M1-1 workspace dependencies, stable toolchain, core module split, and CI concurrency configured.

2026-10-18: M1-4 frame-level async Transport + TCP implementation + loopback integration tests implemented on branch feat/m1-4-transport. Message-level API arrives with M1-3 integration.

2026-10-18: M1-2 review nits: isolated FrameError trait implementations, explained decoder validation, clarified poisoned-frame delivery, and added the 16 MiB payload boundary test.

2026-10-18: M1-5 frame-level reconnect interface, capped exponential backoff with injectable jitter, provisional Resume payload, bounded lifecycle events, and deterministic/unit + loopback recovery tests implemented. Typed Message::Resume integration awaits M1-3; mDNS remains M8.

2026-10-18: PR #16 review: reject oversized sends before connection work and preserve connections on local frame errors; only send I/O errors and write timeouts recover. Cancelled sends drop their transport. Backoff resets on the first received frame, delays have a 1 ms floor, and retry-budget docs count scheduled attempts (including cancelled sleeps). Added regression tests.
2026-10-18: M1-4 review nits: increased the loopback stream timeout, renamed the ping-pong roundtrip test, and documented send allocation, throughput, and sticky receive errors.

2026-10-18: Milestone 1 progress snapshot, historical (main at 35ea74f; later PRs have since merged, see ROADMAP.md). Merged: #12 (M1-2 framing), #13 and #15 (M1-4 TCP transport/loopback), #14 (M1-1 workspace), #16 (M1-5 reconnect). Open at the time: #17 (M1-6 bitrate controller). Not done at the time: M1-3 (typed messages; PR #18 was closed unmerged) and M1-7 (CI hardening/mocks); these have since landed (M1-3 redo, #17, M1-7 and the reader/writer split in #22 are all merged; see the later entries below). Reviewer follow-ups are tracked in the "M1 follow-ups from review" issue and listed below. Milestone 2 is planned in issue #3; follow-up (3) (split reader/writer) was a prerequisite for streaming and is now done.

## M1 follow-ups
- [ ] (1) Send-only `ReconnectingSession` never resets its retry budget when `max_attempts` is `Some`.
- [x] (2, checked against main `core/src/reconnect.rs`: resolved by documentation/behavior, not by recovery) A `TransportError::Closed` returned by a transport send is not treated as recoverable. Only `Io` and write `Timeout` trigger recovery in both `ReconnectingSession::send` and the split writer path; other errors, including `Closed`, retain the transport and are returned to the caller. `Closed` is used as the disconnect reason when a cancelled inline write drops the transport. Original item: Treat `TransportError::Closed` from send as recoverable, or document that it is not.
- [x] (3, done by #22) `&mut self` means recv blocks send during backoff: needs a split reader/writer or channel design before the streaming milestones (M2 prerequisite).
- [x] (4, done by #22) `JitterSource` should be `Send`.
- [x] (5, done on main: PROTOCOL.md states Resume is unauthenticated and clarifies `last_seq == 0`) PROTOCOL.md: state that Resume is unauthenticated (bind to pairing/auth later), clarify `last_seq == 0`, reword "Kind/flags".
- [x] (6, done on main by M1-3: kinds 0xfe/0xff reserved, `PROTOCOL_VERSION` is 2) M1-3 should reserve kinds `0xfe` and `0xff`, and bump `PROTOCOL_VERSION` when the Resume kind is finalized.
- [ ] (7) Add a server-side Resume handler.
- [ ] (8) `lib.rs` hygiene: group `pub mod` lines then re-exports; add docs to `error.rs` and `traits.rs`.

2026-10-18: M2-2 step A implemented: Linux-only host-linux crate, pure-Rust X11 GetImage capture with packed BGRA, monotonic frame metadata, optional pacing, fake-source unit tests, and Xvfb smoke coverage in CI. PipeWire portal capture remains the next PR.
2026-10-18: M2-0 reader/writer split implemented with a bounded command channel, prompt NotConnected during backoff, deadline-bounded writes, shared lifecycle state/events, and Send JitterSource (covers #19 items 3 and 4). Added deterministic backoff/concurrent-task, replacement-transport, cancelled-write, and static Send checks; existing unsplit reconnect and loopback behavior retained.
2026-10-18: M1-3 typed Message enum and input events integrated with bounded bincode encoding and validated frame kinds; kinds 0xfe/0xff reserved for protocol extension/escape; PROTOCOL_VERSION=2; provisional Resume replaced by Message::Resume kind 9 in reconnect sessions.

2026-10-18: M2-1 monotonic clock and caller-supplied offset estimator, TCP loopback latency/RTT bench, Linux netem helper, CI smoke, and docs/LATENCY.md added; release loopback results recorded (64 KiB p99 7.493 ms, 1 MiB p99 18.760 ms); 2 ms unpaced target remains unmet, real Wi-Fi and glass-to-glass measurements pending.
2026-10-18: PR #22 review: merged typed Message::Resume support into split sessions, made cancelled inline reader writes disconnect with Closed, and distinguished queue deadlines from write deadlines with regression tests and cancellation documentation.

2026-10-18: M1-6 ABR controller implemented on branch feat/m1-6-abr.

2026-10-18: PR #17 review: added absolute RTT slack to tolerate LAN jitter and unified ABR feedback with message::NetStats.

2026-10-18: M2-3 GStreamer H.264 codec crate added on feat/m2-3-h264: low-latency x264, live bitrate changes, upstream force-key-unit, optional probed NVENC/VAAPI with software fallback, avdec_h264 decoding, fallible frame APIs and core trait adapters. I420/RGBA input and packed I420 output; PSNR, bitrate, IDR and 1080p timing tests added. Local 1080p encode p50 ~3.9 ms (10 ms target, 40 ms sanity bound). Ubuntu CI installs GStreamer; Windows compiles an empty gated crate. See docs/CODEC.md for API/platform limits and measurements; hardware runtime remains unverified.

2026-10-18: M1-7 done: M1 wrap-up complete, feature-gated mock platform traits and TCP mock pipeline test added, CI doc job enabled on Linux and Windows, and protocol/contribution docs reviewed. M1 done.

2026-10-18: M2-2 step B implemented: optional Linux PipeWire/Wayland capture with ashpd ScreenCast consent, mapped BGRx/RGBx to opaque BGRA conversion, stride/chunk validation, monotonic metadata, bounded frame queue and session cleanup. Default builds require no PipeWire libraries. Fake conversion tests run without the feature; interactive Wayland smoke test is ignored by default. Dedicated Ubuntu Rust 1.85 feature build/clippy/test job added. Live Wayland validation requires a desktop session.

M2-2B validation: native dev libraries installed locally; fmt, default workspace clippy/tests/build, feature build/clippy and all-feature tests pass, including feature verification on Rust 1.85. Windows cross-target all-targets check passes; full cross-linking is unavailable locally because MinGW dlltool is absent. Interactive Wayland smoke remains unrun because this environment has no Wayland session.

2026-10-18: M2-4 host streaming pipeline implemented on feat/m2-4-host-pipeline.
Added the portable mightymonitor-host library and Linux mm-host binary using X11
capture and GStreamer H.264, core split reader/writer sessions, Hello/Resume
handshakes, Ping/Pong, NetStats-driven ABR, and forced keyframes on request and
reconnect. A one-frame latest-capture slot drops replaced/stale frames; exposed
metrics bound queue occupancy and measure capture-to-send-initiation age. Added
two queue unit tests and the three loopback acceptance tests with fake capture
and dummy encoding. Usage, CLI flags, architecture and resume/backpressure limits
are documented in docs/HOST.md. No new third-party packages; reused Tokio (MIT)
and async-trait (MIT OR Apache-2.0). Required workspace fmt, clippy with warnings
denied, mock-enabled tests, rustdoc with warnings denied, and build all pass.
Windows GNU cross-target all-targets cargo check also passes. Live X11-to-viewer
H.264 streaming, Wi-Fi/end-to-end latency, hardware encoding, and Windows linking
or runtime remain unverified; the existing X11 smoke test is ignored by default.
The roadmap tracks M2 as a whole, so its completion status is unchanged.

2026-10-18: M2-3 review follow-ups: codec-gst now gated on target_os = "linux" (empty crate elsewhere), `#![forbid(unsafe_code)]`, decode input/dimension bounds with overflow-checked even-size chroma math, pipeline setup without unwraps, and core `Encoder`/`Decoder` traits return `Result<_, CodecError>` so the adapters no longer panic. NOTICE and docs/CODEC.md note avdec_h264 (libav) is LGPL. Not done (out of scope): exact version pins, serial/looser latency tests, vah264enc migration, hardware encoder verification.

2026-10-18: M2-6 desktop viewer implemented on feat/m2-6-viewer. Added
mightymonitor-viewer / mm-viewer with pinned eframe 0.31.1, Linux codec-gst H.264
I420 decoding, RGB texture presentation, connection controls, lifecycle state,
fps/payload bitrate/RTT overlay and logged decode-to-texture-submission latency.
The GUI feature can be disabled for the headless generic core ViewerSession;
its split ReconnectingSession reader/writer uses the acknowledged session id and
last received sequence for Resume and waits for a keyframe after reconnect.
Bounded ordered encoded input and a latest decoded-picture slot limit backlog.
NetStats and matching Ping/Pong run at approximately one-second intervals.
Windows compiles the GUI and displays an explicit unsupported decoder error.
Added minimal TCP fake-host acceptance tests for reconnect/Resume/keyframe,
feedback cadence, invalid configuration/Closed, and I420 conversion. Updated
Linux CI GUI prerequisites and NOTICE. Also repaired inherited M2-4 call sites
for M2-3's now-fallible Encoder trait, propagating bitrate failures.

Validation: workspace fmt, clippy with warnings denied, tests, build, mock-feature
tests, PipeWire-feature build/clippy, all-feature tests, rustdoc with warnings
denied, Rust 1.85 all-targets workspace check and PipeWire-feature build,
and Windows GNU cross-target
all-targets clippy pass. Live desktop rendering, real host H.264 streaming,
Wi-Fi recovery/latency, physical presentation timing and Windows linking/runtime
remain unverified. Decode-to-present here means texture submission, not scanout.
M2-4 PR #29 is still unmerged; usage, dependency and manual smoke steps are in
VIEWER.md. Windows real decoding, input forwarding and negotiated colorimetry
remain future work; M2 as a whole is not complete.

2026-10-18: M2-4 post-main integration: updated host and test Encoder adapters to
core's Result signatures. Pipeline::on_stats now returns Result; sessions log
rejected live ABR changes and continue at the previous encoder setting, while
initial bitrate failures remain fatal. Recovered poisoned host locks and added
a capture-thread drop guard so backend panic closes/wakes the frame slot.
Added configurable post-handshake idle timeout (--idle-timeout-secs, default 30s),
renewed by received controls including Pings, and a stderr warning for resolved
non-loopback binds without authentication/encryption. Existing tests already
assert the constant max_queue_len == 1 bound. New tests cover bad Hello version,
wrong role, handshake timeout, capture panic termination, and idle timeout/Ping
renewal; existing Resume-first coverage retained. docs/HOST.md updated.
Verified locally: cargo fmt --all -- --check; cargo clippy --workspace
--all-targets -- -D warnings; cargo test --workspace; cargo build --workspace.
All passed, including six host loopback/pipeline integration tests. The X11 smoke
test remains ignored. Live desktop capture/viewing, Wi-Fi latency, hardware
encoding and Windows build/runtime were not revalidated in this follow-up.
Deferred review item (d): validation before pipeline/session resource creation
requires restructuring the current API and CLI capture-dimension setup; invalid
Hello version/role is rejected today, but after resources have been created.
A capture backend that never returns can still retain its thread; idle timeout
ends the session's wait, and in-progress writes remain bounded by write timeout.



2026-10-18: Android viewer implementation added: native Android Activity and
MediaCodec surface decoding, a JNI bridge into the shared Rust protocol, bounded
ordered video queue, Hello/Resume reconnect, configuration/keyframe gating, and
Ping/NetStats feedback. Rust loopback tests cover actual protocol events, reconnect
and shutdown under queue pressure. Android APK/emulator build workflow added;
APK compilation passed for arm64, 32-bit ARM and x86_64. Real PC H.264
playback passed on Android 10, 11, 14 and 16 emulators; physical phone
acceptance remains pending. This milestone does not include phone capture,
remote input, secure pairing, virtual displays or a production-signed release.


2026-10-18: Android screen hosting and light interface milestone added. Android
MediaProjection consent and a foreground capture service feed surface H.264 to
an original Rust protocol server. Native queue bounds, overflow IDR gating,
handshake, timed writes, keepalive, stop lifecycle and bitrate feedback are
implemented. Capture-to-Rust-to-Android-decoder tests passed on Android 10, 11,
14 and 16 emulators. Real desktop H.264 decoding of Android capture also passed on
all four emulator versions. Desktop and Android now separate View/Share with a light neutral
and green palette; technical settings are collapsed. Physical devices, secure
pairing, remote input, extended monitors, concurrent peers and production
Windows packaging remain incomplete.

2026-10-18: Windows desktop streaming implementation added: shared GStreamer H.264
encode/decode enabled on Windows, DXGI capture via d3d11screencapturesrc with
bounded appsink, validated/cropped packed RGBA, CLI monitor selection, and real
viewer decoding/host launch instead of unsupported placeholders. Windows CI
installs the pinned official 1.28.8 SDK with SHA256 validation and verifies codec
plugins before running tests. Linux synthetic-capture and buffer-boundary tests
exercise the common conversion path; a real Windows capture smoke is opt-in.
This is implementation progress, not physical Windows acceptance. Remote input,
secure pairing, virtual displays, and Windows installers remain separate milestones.
Linux Debian alpha packaging is included from main.


2026-10-19: Private session viewing invitations and pinned TLS 1.3 implemented
across desktop and Android. Authentication failures are terminal; there is no
secure-to-plaintext fallback. Android workers can cancel stalled TLS handshakes.
Local workspace tests include real H.264 over TLS, wrong pin/token rejection,
blocking/async interoperability, Android client/server video and stop lifecycle.
Platform CI/emulator validation is tracked in the milestone PR. Persistent device
pairing, remote control, extended displays and physical-device acceptance remain
incomplete; this is a viewing-security milestone, not a finished product.


2026-10-19: Linux host now selects portal-authorized Wayland capture automatically
when built with its default PipeWire feature. A bounded shared latest-frame source
keeps one portal session across viewer reconnects, handles idle desktops and
renegotiates resolution changes. Synthetic lifecycle/crop/resize tests supplement
native builds; interactive compositor acceptance remains pending.


2026-10-19: Remote-control milestone (development branch): optional desktop
keyboard/mouse input, focused-viewer forwarding, and Android accessibility/IME
input are implemented behind local and viewer grants. Wayland uses one system
RemoteDesktop/ScreenCast consent session. Desktop sharing accepts up to four
viewers with one controller at a time; disconnect/revocation releases held input.
Full workspace tests and warning-free clippy passed locally, along with actual
X11 input and two encrypted H.264 viewer sessions with controller handoff.
Android APK builds pass; automated Android control acceptance is currently blocked
by the test service not rebinding after instrumentation restarts the app. This
is not an Android control acceptance pass. Physical Windows/Android, interactive
Wayland input, virtual displays, discovery and production release signing remain
unfinished. See REMOTE_CONTROL.md for permission and platform details.


2026-10-19: Remote-control acceptance follow-up: secure phone control, real
Android button gestures, text entry through the selected optional keyboard, and
revocation passed on Android 10/11/14/16 emulators. Existing PC-to-phone video
and phone-to-desktop H.264 acceptance tests also passed on these versions. Test
setup now grants accessibility after instrumentation restarts the target process
and waits for the actual target editor's input connection. Shipped app permission
behavior is unchanged. Both desktop installer builds passed, including Windows
installation/runtime/uninstallation checks. Physical devices and interactive
Wayland input remain unverified. PR #44 is the remote-control review milestone.

2026-10-19: Experimental Wayland extension is implemented on the separate
`codex/virtual-display` branch after the remote-control milestone. It requests
the portal's Virtual source, checks advertised support and the returned stream
type, reuses encrypted H.264 viewing/control, and exposes Mirror/Extend in Share.
Unsupported X11/Windows extension fails before publishing a viewing invitation.
Local Linux build, clippy, rejection integration test and Windows cross-check pass.
This Linux desktop advertises virtual-monitor support, but actual creation,
streaming and removal require interactive system consent and remain unverified.
No Windows display driver or X11 extension has been added.


2026-10-19: Live KDE Wayland extension acceptance passed after the user granted
system consent: a new virtual monitor appeared in KScreen, a secure desktop viewer
decoded three real H.264 frames, managed-host Stop exited with status zero, and
the virtual monitor disappeared with the original active display set restored.
This test used loopback and view-only sharing. Layout/window movement, virtual
monitor input, other compositors and physical cross-device Wi-Fi remain unverified.


2026-10-20: Opt-in untrusted local presence discovery is implemented on the
development branch for desktop and Android. Advertisements exclude viewing
credentials; a private link is still required to connect. Rust unit tests and
real local multicast advertise/resolve/remove/no-TCP tests passed. Android source
and resources compile; NSD emulator and physical cross-device acceptance remain
unverified. Work continues without waiting for PR creation per the user request.


2026-10-20: Continued development without waiting for PRs: desktop/Android
read-only IPv4 network selection, desktop private-link prefill, Linux URI-scheme
declaration and Windows per-user URI registration are implemented. Links do not
auto-connect or enable control. Startup argument tests and installer association
checks were added. Local Java/resources and warning-free Rust checks pass; full
updated platform/installer/device checks are still running.


2026-10-20: First discovery emulator results: Android 11/14/16 passed all four
video/control/presence tests. Android 10 failed before app installation while
downloading its SDK platform (invalid archive), not in an app assertion. Bounded
SDK-download retry/preflight was added. Continued work adds a desktop Keyboard
panel for Unicode/composed text; the next Android control run includes Hindi and
a check mark in its real editor assertion. Updated tests are pending.


2026-10-20: Development checkpoint f10545d passed all native/MSRV/PipeWire checks,
Windows installer/runtime/uninstall/URI cleanup, Linux alpha.7 packaging and all
four Android emulator versions, including NSD removal and real Unicode text entry.
Further changes fix Android letterbox clicks reusing a stale pointer, remote IME
fullscreen, Stop status and unavailable selected-network handling; active visible
Android viewing now keeps the display awake. Expanded rotation/click/cleanup
regressions are awaiting fresh runs. No new PR was opened per the user request.


2026-10-20: Added Android full-screen viewing with an explicit exit overlay,
aspect-ratio fitting on viewport resize, restored controls/system bars on exit
and disconnect, and live-video enter/exit regression assertions. Main/debug/test
Java compilation passed locally; fresh emulator execution is pending.


2026-10-20: Follow-up usability changes bound desktop discovery to a compact
scrolling list and keep Android Share controls reachable with scrolling on small
or landscape displays. Full-screen transitions explicitly release remote input.
Local Java compilation and workspace Clippy with warnings denied passed.


2026-10-20: Expanded delivery checks: Linux actual package install/reinstall,
installed decoder initialization, desktop entry validation and purge cleanup;
Windows same-version reinstall before runtime/URI/uninstall checks. Updated the
local Linux packaging default to alpha.7. Workflow YAML parses locally; these new
packaging checks have not run yet. Physical upgrades and clean-device acceptance
remain separate.


2026-10-20: Full-screen/live-video checks passed on all four emulator versions in
run 39372769598. The expanded control suite passed Android 10/11 but failed the
rotation precondition on 14/16. The test measured the background MainActivity's
display; it now uses UI Automator display dimensions and explicit test-device
rotation with cleanup. This diagnosis still requires a confirming rerun; rotated
control is not yet accepted on 14/16. No production permission is automated.


2026-10-20: Installer/APK artifact workflows now prepare portable SHA256SUMS and
BUILD_INFO.json with exact source revision, dirty-source indicator, sizes and
hashes. The standard-library generator passed local independent-hash and nested
path checks; YAML parses. No signing credential or certificate was generated.
Fresh workflow execution for manifest packaging remains pending.


2026-10-20: Confirmed checkpoint 27173aa: Android run 39373557186 passed all four
tests on API 29/30/34/36 after using actual device dimensions/rotation in the test.
This includes live full-screen enter/exit, real rotated clicks, stale-letterbox
rejection, Unicode editor entry, revocation and Stop status. Linux actual package
install/reinstall/runtime/desktop validation/purge passed (39373557147), as did
Windows reinstall/runtime/URI/uninstall (39373557232) and CI/MSRV/PipeWire
(39373557265). Physical-device acceptance and Windows Extend remain unfinished.


2026-10-20: Added Android 13+ optional notification permission on first Share so
its existing Stop action can appear. Declining notifications still proceeds to
separate OS capture consent. Emulator capture helpers now exercise notification
denial when that prompt appears. Main/debug/test Java compilation passed; actual
new permission-flow execution awaits the next Android run.


2026-10-20: Implemented Android release-signing configuration and refusal without
publisher credentials. Added CI negative check and genuine release assembly /
apksigner verification using a disposable random test key, not a production key.
The key/test release APK are deleted and excluded from artifact uploads. Gradle
execution is pending; this is not a published or production-signed release.


2026-10-20: Desktop Full screen now hides navigation/connection/status panels,
keeps an explicit exit, releases held input/focus and consumes local Escape before
remote translation. The new Escape regression plus existing Unicode input tests
passed locally; workspace Clippy with warnings denied passed. Platform/UI checks
are pending. The handover's development evidence was consolidated to remove
superseded pending claims. Website FAQ reflects development viewing/discovery.


2026-10-20: Added actual Android JNI ELF load-alignment and debug/release APK
zipalign checks for 16 KiB pages. The new readelf-based checker accepted a real
16 KiB-linked ELF and rejected a 4 KiB-linked ELF locally. Actual Android-library
and APK checks await CI; physical 16 KiB-page hardware is unverified.


2026-10-20: Android a696298 APK job passed unsigned-release refusal, disposable
release signing/apksigner verification and artifact manifests. Android 16 also
passed the actual optional-notification-denial/video/control/discovery suite;
other emulator results are still running. Related viewer documentation was
updated to remove stale single-viewer/X11-only/no-discovery statements.


2026-10-20: Confirmed a696298 Android run 39377927435 succeeded on all four emulator
versions, including optional-notification denial followed by real video/control.
Release-signing guard and disposable signed build also passed. The next checkpoint
adds desktop full-screen behavior and explicit actual 16 KiB ELF/APK checks; those
new platform checks remain to run. No new PR was created.


2026-10-20: The full-screen viewer now retains visible reconnecting status while
hiding ordinary connected controls. Viewer Clippy with warnings denied passed.
Android workflow coverage was focused on its actual headless-viewer/capture/codec
and shared-protocol dependencies; desktop-only UI changes keep their desktop
checks and no longer need unrelated emulator reruns. Workflow YAML parses.


2026-10-20: Checkpoint e468edb passed Android real JNI/debug/release 16 KiB
alignment, signing guard/test signature checks, and all four emulator tests on
10/11/14/16 (39378671813). Desktop CI/MSRV/PipeWire (39378671820), Windows installer
lifecycle (39378671793), and Linux package lifecycle (39378671812) passed. Matching
Windows/Linux/Android artifacts were retrieved locally and independently checked
against SHA256SUMS and BUILD_INFO.json; each reported clean tracked sources.
The small later reconnect-status/filter follow-up awaits its fresh builds.


2026-10-20: Repeat checkpoint 9fe1716 passed native CI and both installers, but
Android control assertions failed intermittently on 10 (initial tap) and 16
(editor focus after rotation); 11/14 passed. Earlier four-version passes remain
historical evidence, not proof this intermittency is solved. Instrumentation now
waits for shown/focused/stable non-pending target bounds before computing a single
remote gesture; it does not retry clicks or bypass actual control assertions.
Expanded viewer playback checks rotate to landscape and back while requiring new
frames. Java test compilation passes; confirming execution is pending.

The current native Linux viewer was also launched using its developer-only idle
snapshot example. It rendered its normal View page successfully and exited without
starting capture or sending input. This is normal-window UI evidence only.


2026-10-20: Added two independent extra control-test executions on API 29/36,
the versions with intermittent failures. Each runs the real grant/gesture/Unicode/
revocation test in a fresh instrumentation launch on the same isolated emulator;
a failed execution fails the job, rather than being retried until green. The
script refuses non-emulator devices and preserves logs. Execution is pending.


2026-10-20: Simplified the desktop idle View page by hiding an unused sharing
bar. Active sharing and sharing failures retain their global indicators; connected
viewing retains its full-screen action. The native Linux snapshot rendered the
revised idle window successfully; viewer Clippy with warnings denied passed.
No capture or input injection was started for this UI check.


2026-10-20: Expanded viewer-rotation tests failed on API 29/34/36 at the generic
input-target readiness precondition (API 30 passed). That precondition is not
appropriate for view-only video. The viewer check now requires a stable visible
full-screen viewport with correct orientation/4:3 fixture aspect ratio, followed
by three new frames; it does not require keyboard focus. Input-gesture checks
retain their stricter focus/layout requirements. Failures now report safe numeric
geometry/state diagnostics. Full-screen entry also dismisses the local keyboard.
Both Java compilations pass. These changes require confirming execution.
Control repetitions run even if a different full-suite test failed, but the
original suite failure still fails the job; no failed assertion is ignored.


2026-10-20: The fresh-instrumentation stress approach exposed Android 16
Accessibility rebind/crash suppression after repeated forced process restarts
(full suite and first repeat passed, second failed before control began). It was
replaced by three independently consented share/control/rotate/type/revoke/stop
sessions within one test process on every emulator version. The first failed
session aborts the test; no failed result is retried into success. This models
ordinary repeated sharing without repeatedly force-stopping an enabled system
service. Initial capture orientation is explicitly portrait for the letterbox
fixture. The old restart script was removed. Java compilation passes; runs pending.


2026-10-20: Confirming Android run 39395701699 passed all four tests on APIs
29/30/34/36, including fresh viewer frames after rotation and three independently
consented control/share/stop rounds per version. CI 39395701831 also passed.
Historical pending/failure entries above describe earlier revisions. Forced-process
Accessibility recovery remains separate and unverified.

2026-10-20: Added read-only Windows screen enumeration, a Share screen picker and
CLI --list-displays/--display. Exact HMONITOR capture checks the selected snapshot
before startup and every frame, rejecting removal/layout/identity changes without
fallback. Explicit selection stays view-only. This is existing-screen selection,
not Windows Extend. Snapshot rejection tests and Windows adapter cross-Clippy
pass; full platform checks and physical multi-monitor acceptance remain.


2026-10-20: Added GETTING_STARTED.md with plain-language setup, consent, control,
extra-display limits and troubleshooting. It is linked from README and bundled
in both desktop installers. Packaging checks verify the guide's presence and
Linux purge cleanup; installer confirmation is pending. Updated older installer
documentation to distinguish implemented input/Wayland extension from physical
acceptance and Windows virtual-display work.


2026-10-20: Native Windows/Linux, MSRV and PipeWire passed in 39399148245
at 893a490, including the new display adapter and exact-screen capture code.
Linux installer/guide presence and purge checks passed in 39399148203.
A follow-up adds vertical scrolling to the desktop Share page so QR/settings/Stop
remain reachable on short windows. Viewer Clippy passes; final layout CI pending.


2026-10-20: The final scrolling Share UI passed Windows/Linux/MSRV/PipeWire CI
39399759531 and Windows/Linux installers 39399759511/39399759517 at 3676811.
Android repeat 39398842473 passed API 30/34/36; API 29 failed the third session's
first remote tap after the first two sessions completed. Cause is unproven.
Added bounded, content-free gesture dispatch/completion/cancellation counters and
debug-only fixture touch counts for diagnostic assertions; test readiness also
checks the active accessibility window against the actual target window. No
failed click is retried. Confirmation is pending; this is diagnostic work, not a
claim that the Android 10 issue has been fixed.


Development follow-up: Added desktop Stop control without interrupting viewing.
Six real framed-socket tests with synthetic input/capture pass, including local
owner loss, revocation across sessions, late-input discard, refusal on reconnect
and held-input release during a stalled video write. Viewer/host Clippy passes.
Native input was not injected during these tests. Platform builds pending.

Android run 39473898336 built successfully and passed API 29/30. API 34 failed on
a stale system-permission UI handle, which blocked later fixture launches; that
helper now re-resolves stale handles within its original timeout. API 36 missed
a rotated valid click in the third session; the rotated assertion now includes
dispatch/target counters. This cause remains unproven, not reported as fixed.


Development follow-up: Implemented explicit Windows screen input mapping using
physical display offsets and virtual-desktop absolute pointer coordinates. Main
screen also pins its snapshot. Odd-edge video cropping is accounted for; changed
or removed display snapshots fail before input, while ReleaseAll remains allowed.
Geometry tests cover monitors left/above the main screen, offsets, corners and
invalid/out-of-desktop coordinates; snapshot tests and adapter cross-Clippy pass.
No input was injected on the developer desktop. Native Windows builds and
physical mixed-DPI/hotplug acceptance remain separate; no virtual driver added.


2026-10-23: Resumed Windows selected-monitor control builds. Android control
failure diagnostics in run 39476317032 showed an accepted DOWN without UP and
a cancelled continued stroke. Quick down/up now coalesces into one complete tap
within a 32 ms window; held presses retain continued dragging. Release clears
scheduled presses. Tests require four additional independent taps each session,
not retries. App Java compilation passes; confirming emulator run pending.
Windows installer run 39951210788 failed before compilation because its official
SDK download ended prematurely. Bounded network retries are added below; pinned
checksum verification and install/plugin failure behavior stay strict.


2026-10-23 follow-up: Windows checks failed before compilation because the
vendor SDK response repeatedly ended prematurely, including bounded PowerShell
retries. Download now uses the runner's curl HTTP/1.1 with timeouts/retries;
pinned checksum verification is unchanged. Android run 39951349198 showed API 29
cancelling one complete 16 ms tap before target DOWN (other APIs passed). Complete
taps now use Android's tap-timeout duration; held drag segments remain short.
This is a timing hypothesis requiring confirmation, not a resolved reliability
claim. Main Java compiles locally. No permission bypass or gesture retry added.


2026-10-23: Added original virtual-output backend/ownership abstraction with mode
validation, exact OS identity, rollback and scoped idempotent cleanup. Four mock
lifecycle tests pass (independent owners/preservation, invalid mode, rollback,
cleanup retry), plus existing snapshot/geometry checks. This is a tested
foundation, not a Windows virtual-display driver or working Extend capability.
