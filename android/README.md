# Android viewer alpha

Native Android UI/MediaCodec with the shared original Rust protocol via a narrow
JNI bridge. This milestone views an existing PC screen; phone capture, remote
input, secure pairing and virtual displays are not implemented here.

Build requirements: JDK 17, Gradle 8.9, Android SDK 35/build-tools 35.0.0, NDK,
Rust Android targets and cargo-ndk 0.8.1. See the Android viewer workflow for the
reproducible build and APK artifact. The debug APK is installable for development,
not a signed production release or Play Store package.

From the repository root:

```sh
cargo install cargo-ndk --version 0.8.1 --locked
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -p 26 -o android/app/src/main/jniLibs build --release -p android-bridge --features jni
cd android
gradle assembleDebug
```

Install app/build/outputs/apk/debug/app-debug.apk on an Android 8+ ARM device
(or x86_64 emulator). Start the PC host on its LAN IP, enter that IP:port in the
app, acknowledge the trusted network and press Connect. Disconnect/backgrounding
stops the Rust worker and decoder. Rotation uses the same surface when possible;
a surface loss closes viewing. Current connections are not encrypted or paired.

Native event queues are bounded and retain encoded reference order. The Rust
worker uses the existing framing/messages, sends Hello/Resume and Ping/NetStats,
reconnects after network failure, waits for configuration and a new keyframe,
and validates dimensions and frame limits. Java only handles UI and Android
MediaCodec. Frame counters measure decoder output submission, not physical
screen scanout. Android platform APIs remain subject to device codec limits.

Physical phone playback, rotation, Wi-Fi recovery and clean installation still
need verification. Emulator smoke jobs check real PC H.264 to Android decoding on API 29, 30, 34,
and 36 (Android 10, 11, 14, and 16). Native libraries use 16 KiB ELF alignment
and AGP 8.7.3 APK packaging for newer Android page-size compatibility.
