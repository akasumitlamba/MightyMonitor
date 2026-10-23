# Extract pinned Microsoft build packages. No driver/certificate installation.
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'This setup is for ephemeral GitHub runners only.' }
$version = '10.0.26100.6584'
$packages = @{
    'microsoft.windows.wdk.x64' = 'c393d03dfb640b5c92f546b32f6770ef68cd3aaf691956e7d66d8e2c28a1b55e'
    'microsoft.windows.sdk.cpp' = '5d31b38205bdd9ac761b4cb39fbbc6b7209b01c11194324afc674d7d119483a0'
    'microsoft.windows.sdk.cpp.x64' = 'c29ce7a4641cb37ee32ebb8078cc65cfbabc7025076bcfba869039204b1e960d'
}
$base = Join-Path $env:RUNNER_TEMP 'mightymonitor-driver-kit'
New-Item -ItemType Directory -Force $base | Out-Null
Add-Type -AssemblyName System.IO.Compression.FileSystem
$roots = @()
foreach ($name in ($packages.Keys | Sort-Object)) {
    $archive = Join-Path $base "$name.nupkg"
    & curl.exe --http1.1 --location --fail --silent --show-error --connect-timeout 30 --max-time 180 --retry 2 "https://api.nuget.org/v3-flatcontainer/$name/$version/$name.$version.nupkg" --output $archive
    if ($LASTEXITCODE -ne 0) { throw "Could not fetch $name" }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne $packages[$name]) { throw "Checksum mismatch for $name" }
    & nuget.exe verify -Signatures $archive -NonInteractive
    if ($LASTEXITCODE -ne 0) { throw "NuGet signature validation failed for $name" }
    $destination = Join-Path $base $name
    if (Test-Path $destination) { Remove-Item -Recurse -Force $destination }
    [IO.Compression.ZipFile]::ExtractToDirectory($archive, $destination)
    $roots += Join-Path $destination 'c'
}
$env:MM_KIT_ROOTS = $roots -join [IO.Path]::PathSeparator
$env:MM_WDK_ROOT = Join-Path $base 'microsoft.windows.wdk.x64/c'
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
if (!(Test-Path (Join-Path $env:LIBCLANG_PATH 'libclang.dll'))) { throw 'Runner LLVM/libclang is missing.' }
foreach ($entry in @('MM_KIT_ROOTS','MM_WDK_ROOT','LIBCLANG_PATH')) {
    "$entry=$([Environment]::GetEnvironmentVariable($entry))" | Out-File -FilePath $env:GITHUB_ENV -Append -Encoding utf8
}
