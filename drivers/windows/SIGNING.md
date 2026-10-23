# Driver signing handoff

The user confirmed on 23 October 2026 that the upstream maintainer can handle
Microsoft driver signing. No credentials or private signing material belong here.
No message/submission to the maintainer or Microsoft is automated by this workflow.

This is an **unsigned development candidate**, not an approved app release.
Review the source revision, lifetime changes and native build results first.
The app Share flow does not yet use this component. Signing does not establish
that creating, capturing, controlling and removing an output works on hardware.

The dedicated workflow produces MightyMonitorDisplay.dll, its INF, an unsigned
catalog, public license/provenance, source revision and SHA-256 manifest. Inf2Cat
checks the catalog for Windows 11 24H2 x64; other Windows versions need the
maintainer's matching validation/acceptance. Microsoft documents the tool at
https://learn.microsoft.com/windows-hardware/drivers/devtest/inf2cat.

Use the maintainer's established Microsoft-accepted signing process to obtain a
signed candidate for native-device acceptance. Do not change DLL/INF bytes after
catalog generation/signing. Return only public signed driver files and their
source revision. Preserve the included license/provenance with distribution.
The project must validate returned signatures/checksums and complete create,
rollback, stop, disconnect, process crash, sleep/resume and device removal tests
before enabling normal Windows Extend or publishing a stable installer.

Driver installation must remain explicit and separate from per-user viewing.
Never install a root certificate, disable enforcement, turn on test-signing or
silently download/execute a driver. The ordinary app installers currently include
no driver. Physical-device testing remains deferred by the user's request.
