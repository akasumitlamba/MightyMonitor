# MightyMonitor static website

Plain HTML, CSS, JavaScript, images and bundled fonts. No build step, framework, application server, account or hosting-service dependency.

## Open on your computer

Double-click `dist/index.html`, or open it from your browser's File menu. All styles, scripts, images and fonts use relative paths, so this works directly from disk. Internet is only needed to look up releases or follow project links. Without connectivity, the page remains readable and shows a link to check GitHub Releases later.

For an optional local server, run from the repository root:

```sh
python3 -m http.server 8765 --directory website/dist
```

Then open http://localhost:8765.

## GitHub Pages

The repository includes `.github/workflows/website.yml`, which uploads `website/dist` directly. No package installation or build command is needed.

1. Merge this branch into the repository's main branch.
2. In GitHub, open **Settings → Pages** and choose **GitHub Actions** as the source.
3. Open **Actions → Publish project website → Run workflow**. Later changes to `website/dist` on main deploy automatically.
4. GitHub displays the website address on the successful deployment. A project repository typically uses `https://OWNER.github.io/MightyMonitor/`.

All asset paths are relative, so the page supports GitHub Pages repository subpaths. The workflow uses the repository where it runs; download discovery intentionally points to the original application's releases at `reduckyLang/MightyMonitor`. Change `PROJECT` in `dist/app.js` only if you intentionally distribute installers from another repository.

## Real installer links

The page requests the latest stable release from the public GitHub API. It only links to installable `.msi`, setup/installer `.exe`, `.deb`, `.rpm`, or `.apk` assets belonging to the configured repository's release-download path. Bare executables, portable archives and source archives are excluded. Offline/rate-limited lookups provide a manual Releases link. Missing releases/packages leave controls disabled.

The website does not create installers. Maintainers must build, validate and attach packages to a published stable release before download links appear. No credentials are embedded.

## Checks and content

```sh
node website/tests/downloads.cjs
```

The automated tests cover missing releases, real installer filtering, untrusted URLs, source-only releases and offline/rate-limit fallback. The development status is an editorial snapshot dated 20 October 2026. Update it at every merged milestone, keeping implementation, emulator/local validation and physical-device acceptance distinct. The interactive device illustration is explicitly a concept, not a screenshot or a claim that all platform/device combinations work. Icons are inline SVG and do not depend on font glyph coverage.

See DESIGN.md for the frontend-design skill's design plan and critique. The icon is original project artwork. Bundled Sora and IBM Plex Sans fonts include their open-font licenses in `dist/assets/fonts/`.
