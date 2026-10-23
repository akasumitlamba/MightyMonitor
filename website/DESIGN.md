# Design: devices in the same orbit

Reviewed 23 October 2026. Uses the installed frontend-design skill. The site is
plain HTML/CSS/JS; it needs no package installation, framework, hosted preview or
build step. Original CSS and inline SVG create the illustrations.

## Direction and tokens

The product is about relationships between physical screens. Give those screens
real visual weight: perspective, layered bezels, cast shadows and soft reflected
light. Animate the connections and screen content so the visitor understands
what moves between devices. Keep the surrounding reading experience quiet.

- Porcelain `#f8faf8`: reading surface.
- White `#ffffff`: windows and download surfaces.
- Forest `#256a4c`: primary action and selected mode.
- Mint `#dff3e7`: connection and screen illustration.
- Ink `#1b3028`: headings and body.
- Sage `#5c6d65`: secondary text.
- Sora: restrained, medium-weight display type; whole headlines share one style.
- IBM Plex Sans: body and controls. Both families are bundled with licenses.

```
brand                              progress / downloads / GitHub

Your screens.                 dimensional Linux + Windows + phone
Better together.               animated local connection paths
purpose + download                 Mirror / Control / Extend

three illustrated possibilities: mirror / control / extra desktop
current platform status               honest development timeline
Windows EXE                  Linux DEB                 Android APK
practical questions                                      footer
```

Desktop: left-aligned copy and one substantial scene. Tablet: centered opening,
then illustrated horizontal feature rows. Phone: stacked composition and compact
installer cards. Main navigation and reading content stay available without JS.

## Critique and revision

The previous layout placed small, mostly static devices below the heading and
used thin separators everywhere. It gave neither the product nor the graphics a
clear focal point. This pass moves the device composition beside the copy and
replaces icon-only feature rows with short visual demonstrations. Perspective,
material edges and shadows carry the depth; no stock 3D render or WebGL download
is necessary. The neutral/green direction respects the requested app identity.

Motion has a purpose: moving signals suggest local transport; the Control cursor
shows interaction; a window crosses displays in the Extend illustration. Slow
floating establishes depth. Mode switching is user-driven; it never auto-rotates.
A global pause control stops looping animation. System reduced-motion preferences
remove animation and smooth scrolling; a hidden page also pauses animation.
Without JavaScript the artwork starts paused and the inactive pause control is
hidden. There is no drawing loop, telemetry, permissions request or CDN dependency.

## Product truth and maintenance

The scene is a concept, not a live app screenshot. Keep alpha labels and platform
limitations visible. Do not imply Windows Extend is ready because a driver builds.
Installer links still resolve only actual assets in the latest stable upstream
release. EXE/DEB/APK descriptions match the current packaging configuration.

Update the editorial snapshot as implementation progresses. Keep these files
relative-path compatible with both file:// and GitHub Pages repository subpaths.
Use inline SVG rather than font glyph icons. Version asset query strings when
changing cache-sensitive styles/scripts. Do not add Sites hosting or remote fonts.
