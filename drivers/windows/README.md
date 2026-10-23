# MightyMonitor Windows display driver — development only

This isolated workspace is an **experimental unsigned driver component**, not a
working Windows Extend feature in the application. Do not install it on a daily
computer. The app does not load, download, trust or install this driver.

The UMDF/IddCx, EDID and swap-chain implementation is derived from the MIT-licensed
`MolotovCherry/virtual-display-rs` at commit
`22fcd2e065aac13745cb27dc869848091b9e3a23` (copyright 2024 Cherry).
`LICENSE.virtual-display-rs` preserves the license. `PROVENANCE.json` records the
imported files and original hashes. This component is an attributed open-source
adaptation; it is not represented as independently authored driver code. The
MightyMonitor screen-sharing engine and protocol remain original implementations.

MightyMonitor replaces the original global monitor-list IPC with its own bounded,
per-connection Create/Check/Remove/Ping protocol. Each connection owns at most one
monitor and cannot list, change or remove another connection's output. Disconnect,
idle timeout or malformed framing drops its lease. Failed departure is retried
without reusing the connector slot. The driver returns the actual OS adapter LUID
and target ID, not a connector index. A dedicated pipe and hardware ID keep it
separate from the original driver. Local interactive users may connect; remote
pipe clients are rejected. A host adapter must authenticate the pipe server before
trusting its output identity. The native endpoint is not yet wired to the app.

The reference's event-log registry dependency/startup retry is removed. Native
creation uses a ten-second response deadline; a late successful task result drops
its lease and rolls back. A Windows framework operation cannot safely be forcibly
terminated inside the driver. Cleanup failure/recovery, concurrency, hotplug and
actual Windows topology/input still need native runtime testing. Neither protocol
unit tests nor a DLL build prove these behaviors on Windows.

Build with the dedicated workflow and Microsoft SDK/WDK 10.0.26100.6584 packages.
The package installer extracts verified build dependencies into the ephemeral
runner, without installing certificates or changing signature enforcement. Never
ship an unsigned/test-signed driver as a production installer. Production signing,
INF/catalog verification and the explicit installation/update path remain work.

Local portable protocol checks:

```
cargo test --manifest-path drivers/windows/Cargo.toml -p mm-display-driver-protocol --locked
```

Keep this workspace separate from ordinary app builds. Driver binaries, SDKs,
private keys, certificates and generated bindings must not be committed.
