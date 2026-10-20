# Sign an Android release

Debug APKs are for development. Their per-run debug key does not provide stable
update continuity. A distributable APK needs a private signing key which the
publisher retains securely for future updates. No private key is in this repo.

Build the Rust JNI libraries as described in README.md. In the build environment,
set all four variables using a secret manager or local environment configuration:

- `MM_ANDROID_KEYSTORE`: absolute path to the publisher's private JKS/PKCS12 file.
- `MM_ANDROID_STORE_PASSWORD`: its store password.
- `MM_ANDROID_KEY_ALIAS`: the signing key's alias.
- `MM_ANDROID_KEY_PASSWORD`: that key's password.

Then run `gradle -p android assembleRelease` from the repository root. Missing
values or a missing file fail the release build; there is no unsigned or debug-key
fallback. The result is `android/app/build/outputs/apk/release/app-release.apk`.
Verify it with the Android SDK's `apksigner verify --verbose <APK>`. Retain the
same application ID, signing key and an increased versionCode for later updates.
Changing the key normally requires uninstalling the old app and losing its data.

Keep the key and passwords outside the repository, artifacts, shell history and
logs. Back up the key securely; losing it can prevent updating existing installs.
Do not place passwords in command arguments or commit local signing properties.
Production release publication remains a separate maintainer action.

CI checks both refusal without signing credentials and actual release assembly/
signature verification using a random, short-lived disposable test key. It deletes
the key and signed test APK and uploads neither. That test establishes the build
path, not production signing identity, physical-device acceptance or a published
release. The normal CI downloadable APK remains clearly a debug/test build.
