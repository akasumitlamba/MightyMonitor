# Android sharing and viewer alpha

Native Android UI/MediaCodec with the shared original Rust protocol via a narrow
JNI bridge. View an existing PC screen or explicitly share the phone screen.
Remote input, secure pairing and virtual extended displays are not implemented.

Build requirements: JDK 17, Gradle 8.9, Android SDK 35/build-tools 35.0.0, NDK,
Rust Android targets and cargo-ndk 4.1.2. See the Android viewer workflow for the
reproducible build and APK artifact. The debug APK is installable for development,
not a signed production release or Play Store package.

From the repository root:

```sh
cargo install cargo-ndk --version 4.1.2 --locked
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -P 26 -o android/app/src/main/jniLibs build --release -p android-bridge --features jni
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

## Share the phone screen

Open Share, confirm a trusted Wi-Fi network and choose Share this screen.
Approve Android's system capture prompt for this session. On a PC, open View
and enter the address shown on the phone. End sharing in the app, its ongoing
notification, or Android's screen-sharing controls. Denied permission never starts
capture. A stopped or killed service does not restart capture automatically.

MediaProjection supplies a surface to the device's H.264 MediaCodec encoder.
Output uses a fixed canvas up to 1280 pixels on the longer edge, respecting
encoder alignment; rotated content can be letterboxed. The service owns its
projection callback, encoder, surface, virtual display and Rust host worker.
Capture continues in a visible foreground service while the app is backgrounded.
It shares video only; no microphone or audio is captured.

Rust owns the original TCP/framing/message server. It handles Hello/Resume,
configuration, sequence numbers, Ping/Pong, timed writes and bounded bitrate
feedback. Only one viewer is served at a time. Queue overflow discards the old
reference chain and requests a fresh keyframe, including SPS/PPS. A new viewer
always starts on a keyframe. Sharing remains unauthenticated and unencrypted;
use a trusted local network. Control is not enabled by this implementation.

Both PC-to-phone playback and consent-based phone capture passed emulator tests
on Android 10, 11, 14 and 16. The desktop decoder interoperability check is a
separate acceptance step; physical Wi-Fi phone/PC verification remains pending.
