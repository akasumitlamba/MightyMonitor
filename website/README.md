# MightyMonitor project website

Static HTML, CSS, and JavaScript in `dist/`. No build step, framework, API keys, or server runtime required.

## Preview

From the repository root:

```sh
python3 -m http.server 8765 --directory website/dist
```

Open http://localhost:8765. Run the download behavior tests with:

```sh
node website/tests/downloads.cjs
```

## Downloads

The page requests the latest stable release from the public GitHub API for `reduckyLang/MightyMonitor`. It only displays links to installable `.msi`, setup/installer `.exe`, `.deb`, `.rpm`, or `.apk` assets belonging to that repository's release-download path. Bare executables, portable archives, and source archives are excluded. A failed/rate-limited lookup gives a link to GitHub Releases. No release or no installer means disabled controls, not fabricated downloads. No credentials are embedded in the website.

This website does not create installers. Maintainers must build, validate, and attach installer packages to a published stable release before download links can appear. Review code-signing and distribution requirements separately.

## Status and design

Development status is an explicitly dated editorial snapshot (18 October 2026), not inferred from commit counts. Update this content when physical-device acceptance changes. The interactive screen illustration is a concept preview, not a screenshot or a claim of working Android/control/extension support.

The frontend-design skill from Anthropic was installed and applied to the revised design. See DESIGN.md for the palette, typography, layout, and critique. `assets/icon.png` is the original project icon. Fonts are loaded from Google Fonts with local sans-serif fallbacks.

## Hosting

Deploy `dist/` with any static host. `.openai/hosting.json` stores the private Sites preview identity and static directory; it contains no credentials. A new deployment uses the Sites workflow. Keep GitHub source contributions separate from the hosted preview's own source repository.
