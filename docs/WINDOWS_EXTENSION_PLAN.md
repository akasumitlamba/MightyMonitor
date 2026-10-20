# Windows extended-display implementation plan

Research snapshot: 20 October 2026. Windows virtual-monitor creation is **not
implemented** in MightyMonitor. Existing DXGI mirroring is implemented. Do not
rename an existing-screen capture as Extend or claim an unsigned driver is a
production-ready installer.

## Required contract

A backend must create an independent Windows desktop output, return an exact
monitor identity/handle and desktop rectangle, stream only that output, and remove
only its own monitor when sharing stops. Validate cleanup on errors, reconnect,
application exit and crashes. Existing monitors and their layout must survive.
Add display-layout/resolution controls only after ownership and targeting work.

The current host accepts a GStreamer monitor index, but controller input remains
limited to the primary/default monitor. An index inferred from enumeration order
is not a safe way to identify a newly created virtual display during hotplug.
Use a verified monitor identity and the capture source's `monitor-handle` property.
Map normalized input into that display's full desktop rectangle, including negative
coordinates and DPI changes. The current Enigo Windows absolute-pointer path uses
primary-display dimensions without MOUSEEVENTF_VIRTUALDESK; it cannot simply be
reused with a secondary-display offset. Keep the existing control restriction
until the replacement path is implemented and tested.

## Compatible Rust driver research

[virtual-display-rs](https://github.com/MolotovCherry/virtual-display-rs) is an
MIT-licensed Rust Windows indirect display driver. It is a possible optional
backend when already installed by the user, not an incorporated dependency yet.
No driver code, binary, certificate or installer has been added to MightyMonitor.
Its public `rust/driver-ipc` API uses the `virtualdisplaydriver` named pipe,
JSON messages delimited by byte 0x04, State/Notify/Remove commands and monitor
IDs/modes. Review and pin a release/commit before implementing compatibility.

Important correctness details from the public source:

- Notify replaces the complete monitor list, removing omitted monitors. A stale
  read/append/write can overwrite another application's concurrent changes.
  Do not blindly send a one-monitor Notify or assume it is an atomic Create.
  Prefer a backend/protocol with atomic owned Create/Remove or obtain a documented
  exclusive-driver contract; tests must cover concurrent changes and preservation.
- Remove accepts individual monitor IDs. Cleanup must never use RemoveAll.
- Its EDID serial identifies the monitor ID. Its configurable monitor name is not
  sufficient evidence of the actual captured Windows monitor identity.
- The upstream installation currently requires explicit certificate trust. Do not
  automatically add a root/TrustedPublisher certificate, turn off signature
  enforcement, enable test-signing, or download/run a driver silently.

Relevant upstream source:
[IPC contract](https://github.com/MolotovCherry/virtual-display-rs/blob/master/rust/driver-ipc/src/core.rs),
[driver Notify implementation](https://github.com/MolotovCherry/virtual-display-rs/blob/master/rust/virtual-display-driver/src/ipc.rs),
[monitor identity](https://github.com/MolotovCherry/virtual-display-rs/blob/master/rust/virtual-display-driver/src/context.rs).
These are reference material under their license, not proprietary application code.

## Delivery and acceptance

1. Implement a monitor-backend abstraction and mock owned creation/removal,
   concurrent changes, timeouts, failure rollback and monitor identity checks.
2. Choose/build the Rust driver and its safe IPC contract. Keep installation
   separate from unprivileged viewing/sharing. Record dependency provenance.
3. Add exact Windows monitor targeting and secondary-display pointer mapping;
   unit/mock tests and cross-compilation cannot establish physical correctness.
4. Test real Windows 11: extra desktop space, moving a window onto it, Linux and
   Android viewing/control, DPI/resolution changes, hotplug, cleanup and restart.
5. Establish legitimate production driver signing and installation/update paths
   using the maintainer's signing/account resources. Never commit credentials or
   label a test certificate as Microsoft production approval.
6. Test opposite-direction Linux↔Windows extended streams plus a phone session.

References:
[GStreamer monitor targeting](https://gstreamer.freedesktop.org/documentation/d3d11/d3d11screencapturesrc.html),
[Microsoft driver signing](https://learn.microsoft.com/windows-hardware/drivers/dashboard/code-signing-reqs).
