# Real-device acceptance

This checklist is for Windows 11, a Linux desktop and physical Android 10, 11,
14 and 16 phones. Emulator results do not fill these rows. Record the exact
commit, package checksum, OS version and observed outcome; leave untested rows
untested. Keep private viewing links, device addresses and personal screenshots
out of committed reports.

## Install and connect

1. Install the matching development installer on each device. These are alpha
   builds. Windows setup downloads its verified video runtime on first install;
   Linux packages need their declared distribution libraries. Android test APKs
   may require uninstalling an earlier build if its debug signing key differs.
   Uninstalling removes local app settings. Production update signing is unfinished.
2. Put the devices on the same private Wi-Fi. In Share, choose the correct network
   address, start sharing and approve the operating system's capture prompt.
3. Copy the private link to View on the other device and press Connect. Treat the
   link as a password. Nearby discovery shows presence; it does not grant access.
4. Confirm readable text and moving video for 15 minutes. Record disconnects,
   frozen frames, heat and battery behavior. Repeat with the roles reversed.
5. Android: enter/exit Full screen, rotate both devices, leave/reopen the app,
   stop sharing via the notification and restart. Foreground viewing stays awake;
   manual lock and protected content retain OS restrictions.

## Control and recovery

- Confirm the viewer cannot control a view-only share. Enable local control and
  explicitly request Control on the viewer. Android Accessibility/keyboard setup
  and Linux portal dialogs require the person using that device.
- Tap/click all four corners, type normal and composed text, use Shift, scroll,
  revoke control while a key is held and verify it is released. Check after
  rotation and letterboxing; black bars must never click a stale position.
- Turn Wi-Fi off then on. Verify video recovery, fresh control authorization,
  and no replay of old keys or touches. Repeat Stop/Share with a fresh link;
  the old link must fail.
- Connect two viewers to one desktop. Only the granted controller should inject
  input; disconnecting the other viewer must not release the controller's keys.

## Extra display and wired links

Linux Wayland Extend is experimental and compositor-dependent. Approve a new
virtual monitor when offered, confirm it appears as a separate output in system
Display settings, move a window onto it and see that window on the other device.
Check that Stop removes only the created output. Virtual-display input and layout
still need acceptance. Windows and X11 true extension remain unimplemented.

Repeat basic sharing over Ethernet or Android USB networking after selecting the
new interface and creating a fresh link. A normal USB-C cable between laptops
is insufficient; test USB4/Thunderbolt networking only on supported hardware.
See NETWORK_SETUP.md. Do not record zero-latency claims: measure motion delay
with an external high-frame-rate camera and report median/p95 plus method.

| Route | Mirror/video | Control/revocation | Recovery | Sustained session | Extra display |
| --- | --- | --- | --- | --- | --- |
| Windows → Linux | Untested | Untested | Untested | Untested | Missing implementation |
| Linux → Windows | Untested | Untested | Untested | Untested | Wayland acceptance pending |
| Windows → Android | Untested | Untested | Untested | Untested | Windows implementation missing |
| Linux → Android | Untested | Untested | Untested | Untested | Wayland acceptance pending |
| Android → Windows | Untested | Untested | Untested | Untested | Not an Android host feature |
| Android → Linux | Untested | Untested | Untested | Untested | Not an Android host feature |

Run each Android route separately for versions 10, 11, 14 and 16. Then test
opposite-direction simultaneous sharing, watching for recursive screen capture
and control loops. Keep each session's permission and disconnect independent.
