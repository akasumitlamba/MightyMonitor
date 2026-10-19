# Pinned official GStreamer SDK for native Windows CI. No unpinned package feeds.
$ErrorActionPreference = 'Stop'
$version = '1.28.8'
$expectedHash = 'e4c41ee6ee9c6c32a45d198463ec08d79bac3bd5fa58adc93d13fe4a08abc9d9'
$filename = "gstreamer-1.0-msvc-x86_64-$version.exe"
$installer = Join-Path $env:RUNNER_TEMP $filename
$root = Join-Path $env:RUNNER_TEMP 'gstreamer'
Invoke-WebRequest "https://gstreamer.freedesktop.org/data/pkg/windows/$version/msvc/$filename" -OutFile $installer
$actualHash = (Get-FileHash $installer -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualHash -ne $expectedHash) { throw 'GStreamer installer checksum mismatch' }
$process = Start-Process -FilePath $installer -ArgumentList @('/VERYSILENT', '/NORESTART', '/CURRENTUSER', '/TYPE=devel', "/DIR=`"$root`"") -Wait -PassThru
if ($process.ExitCode -ne 0) { throw "GStreamer installation failed: $($process.ExitCode)" }
$bin = Join-Path $root 'bin'
if (-not (Test-Path (Join-Path $bin 'gst-inspect-1.0.exe'))) { throw 'GStreamer installation is incomplete' }
$env:PATH = "$bin;$env:PATH"
$env:PKG_CONFIG_PATH = Join-Path $root 'lib/pkgconfig'
$env:GSTREAMER_1_0_ROOT_MSVC_X86_64 = "$root\"
$bin | Out-File -FilePath $env:GITHUB_PATH -Encoding utf8 -Append
"PKG_CONFIG_PATH=$env:PKG_CONFIG_PATH" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
"GSTREAMER_1_0_ROOT_MSVC_X86_64=$root\" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
# Fail rather than silently skip the runtime codec tests if plugins are missing.
foreach ($plugin in @('appsrc', 'appsink', 'videotestsrc', 'videoconvert', 'x264enc', 'avdec_h264', 'd3d11screencapturesrc')) {
    & (Join-Path $bin 'gst-inspect-1.0.exe') $plugin | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Required GStreamer plugin missing: $plugin" }
}
