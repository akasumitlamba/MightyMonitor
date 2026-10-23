# Using MightyMonitor

MightyMonitor is an alpha app for sharing a screen with another Windows, Linux
or Android device on your local network. Development installers are available
from the project's build workflows. A successful build is not a stable release.

## Connect two devices

1. Install and open MightyMonitor on both devices. Connect them to the same Wi-Fi
   or an existing local Ethernet/USB network.
2. On the device you want to see, choose **Share**. Choose the correct
   network if it is not already selected (desktop Settings or Android Network). Start sharing and approve the system's
   screen-sharing prompt when it appears.
3. Copy the private viewing link to the other device. Open **View**, paste the
   link and press **Connect**. Opening a MightyMonitor link only fills this field;
   it does not connect automatically.
4. Use **Full screen** for a larger picture. Exit using the on-screen exit button,
   Escape on desktop, or Back on Android. Stop sharing on the source device when
   finished. A new sharing session has a new private link.

The private link grants viewing access. Share it only with the intended viewer.
Optional **Show on local network** helps nearby devices find the sharing address;
it does not give them the private link or control permission.

## Control the other screen

Before starting Share, enable **Allow remote control** on the source device.
The viewer must also request control. Click or tap inside the viewed picture to
send input. Use the viewer's **Keyboard** control for text entry. Only one desktop
viewer can control a host at a time; other viewers can watch.

On a desktop viewer, keyboard input follows focus in the picture. Moving focus
away releases held input. Reconnecting needs a fresh control grant. Use **Stop control** on the source desktop's Share page to revoke control while
video continues. Start a new sharing session to enable control again. Stop sharing
ends viewing and control together.

When sharing an Android screen, the phone owner must enable MightyMonitor's
Accessibility service in Android Settings. To receive typed text, also enable
and select the optional MightyMonitor keyboard. Android controls those settings;
the app does not enable them automatically. Android may restrict protected screens
or certain system actions. Notification permission is optional; screen-capture
permission is separate.

Windows can share its main screen or a screen selected in the Share page. Optional
control follows that screen; this new path still needs physical multi-monitor
acceptance. Wayland asks for input permission through its system portal.
If you started without control permission, stop sharing before enabling it.

## Use another device as an extra monitor

On a supported Linux Wayland desktop, choose **Extend**, then **Share extra
display**, and approve a new/virtual monitor in the system dialog. Open its private
link on the receiving device. Arrange the new monitor in Linux Display Settings,
then move application windows onto it. Stop sharing to remove the extra output.

This has been tested locally on KDE; other desktops and physical-device input
still need acceptance. Windows and Linux X11 extra-display creation are not
implemented. Mirroring a screen does not create extra desktop space. Android can
view an extra Linux display, but sharing Android sends its existing phone screen.

## If a connection does not work

- **No nearby device:** discovery is optional. Paste the private link directly.
  Check that both devices use the same reachable network; guest Wi-Fi may isolate
  devices. Network selection does not configure adapters or change router settings.
- **Connection fails:** start a fresh Share and copy its new link. On Windows,
  allow MightyMonitor through the firewall for the intended private network.
- **Sharing fails:** approve the screen-sharing dialog. On Wayland, try Mirror if
  your desktop does not offer a virtual monitor. Expand **Sharing details** for
  the error. On Windows, refresh screens after reconnecting/changing a monitor.
- **Video works but control does not:** check both control grants, click inside
  the picture, and check the source device's OS permissions described above.
- **Phone text does not arrive:** select MightyMonitor's keyboard on the phone
  and focus an editable field before sending text.
- **Delay or stuttering:** lower frames per second or video quality in Share
  Settings, then start again. Wi-Fi quality, capture and encoding all affect delay.
- **Cable does nothing:** ordinary USB-C ports/cables do not necessarily provide
  a network between computers. Use Ethernet or supported USB4/Thunderbolt networking;
  Android USB networking/tethering depends on the devices and OS configuration.

When reporting a problem, include device types, OS versions, direction of sharing,
and the visible error. Do not include private links or personal screen content.
