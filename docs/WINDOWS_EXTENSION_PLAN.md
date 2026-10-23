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

The host now pins main/explicitly selected screens and maps optional pointer
input through the entire Windows virtual desktop. Legacy numeric monitor indices
remain view-only. An index inferred from enumeration order is not a safe way to
identify a newly created virtual display during hotplug.
Use a verified monitor identity and the capture source's `monitor-handle` property.
The original display adapter's pointer path uses absolute virtual-desktop mapping
with the display's physical rectangle, including negative coordinates and the
codec's odd-edge cropping. Input rechecks its pinned snapshot before applying
events; held input can still be released if the display disappears. Unit geometry
and cross-build checks are evidence, but physical mixed-DPI, hotplug and Windows
11 input acceptance remain required before calling this release-ready.

## Compatible Rust driver research

[virtual-display-rs](https://github.com/MolotovCherry/virtual-display-rs) is an
MIT-licensed Rust Windows indirect display driver. It is a possible optional
backend when already installed by the user, not an incorporated dependency yet.
No driver code, binary, certificate or installer has been added to MightyMonitor.
Its public `rust/driver-ipc` API uses the `virtualdisplaydriver` named pipe,
JSON messages delimited by byte 0x04, State/Notify/Remove commands and monitor
IDs/modes. The read-only audit pins commit `22fcd2e065aac13745cb27dc869848091b9e3a23`;
compatibility is not implemented.

The original `display-windows` adapter now enumerates existing GDI screens and
checks exact handle/device/geometry snapshots for capture and explicitly granted
input. It is a
prerequisite, not driver integration or permission to select an unrelated screen.

Important correctness details from the public source:

- Notify replaces the complete monitor list, removing omitted monitors. A stale
  read/append/write can overwrite another application's concurrent changes.
  Do not blindly send a one-monitor Notify or assume it is an atomic Create.
  Prefer a backend/protocol with atomic owned Create/Remove or obtain a documented
  exclusive-driver contract; tests must cover concurrent changes and preservation.
- The pipe rejects remote connections but uses a NULL DACL allowing local clients;
  it has no per-client monitor ownership. Its accumulated message buffer is not
  bounded by its 4096-byte read chunk, and it accepts unlimited pipe instances.
  An adopted/forked backend needs bounded messages/connections/timeouts and an
  atomic owner lease rather than trusting every local sender.
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

1. The original `display-windows::owned` abstraction now implements validated
   modes and owned lifecycle/rollback. Mock tests cover independent owners,
   preservation, invalid identity rollback and cleanup retry. Native deadlines,
   driver disconnect cleanup and actual concurrent OS operations still need a
   concrete backend and acceptance.
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


## Exact owned-output identity

Microsoft's [IddCx monitor-arrival output](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/iddcx/ns-iddcx-idarg_out_monitorarrival)
returns `OsAdapterLuid` and `OsTargetId` for identifying an arrived output from a
companion app. The audited Rust driver's context currently discards this output.
A dedicated backend should return the pair in its atomic owned-create response;
do not equate connector index with the OS target ID. Match the pair through
QueryDisplayConfig to a source GDI name, then to the enumerated HMONITOR and
rectangle. Reject inactive/cloned output rather than changing all other monitors
to a new topology. Driver signing, lifecycle cleanup and secondary-screen input
still require implementation and Windows testing.


The native `resolve_output(OutputId)` prerequisite is now implemented using
[QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)
and [the GDI source-name query](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_source_device_name).
It bounds allocations/retries and rejects an inactive, cloned or ambiguous owned
output. Synthetic identity tests and Windows-target Clippy pass. It changes no
OS display configuration and is not yet connected to a creating driver. Driver
arrival/OS topology settling and later hotplug need native integration acceptance.
