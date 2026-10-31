# Velopack's release of the app, on Windows, for one architecture: target\package\windows\<arch>\releases\
# (nettai-<channel>-Setup.exe, the installer; nettai-<channel>-Portable.zip, the app alone; and the update package and
# its feed the installed app's updater reads from the GitHub release: nettai-<version>-<channel>-full.nupkg,
# releases.<channel>.json, assets.<channel>.json, RELEASES-<channel>).
#
#   pwsh packaging\windows\package.ps1 [-Arch x64|arm64] [-Version <version>] [-Icon <nettai.ico>] [-Ffmpeg <ffmpeg.exe>]
#
# Each architecture is its own package on its own Velopack channel, named as Velopack names the runtime (`vpk pack
# --runtime`), as a Mac's are, so that each one's files and feed sit beside the other's in one release. Each is built
# natively, on a machine of its architecture:
#
#   x64      win-x64      nettai-win-x64-Setup.exe, nettai-win-x64-Portable.zip
#   arm64    win-arm64    nettai-win-arm64-Setup.exe, nettai-win-arm64-Portable.zip
#
# x64's app is packed a second time, the same folder on the channel `win`, the only one up to 0.19.0
# (nettai-win-Setup.exe, nettai-win-Portable.zip, nettai-<version>-full.nupkg, releases.win.json, assets.win.json,
# RELEASES): the bridge, so that the apps installed from that channel, which ask it alone, find the update, and move to
# win-x64 at the one after (docs/app.md §19, "Updates"), as the Mac's universal channel osx was kept for its apps up to
# 0.18.1. It goes when the user says.
#
# The app: nettai.exe, ffmpeg.exe, LICENSE.txt, THIRD-PARTY-NOTICES.txt, ffmpeg-NOTICE.txt. No content, no ROM and no
# extracted asset is in it: the app downloads each game's content (docs/design/content-deployment.md §5) and extracts
# each game's pack from the player's own copy. The installer is the
# player's alone (%LOCALAPPDATA%\nettai, no administrator), with a Start menu and a desktop shortcut; uninstalling
# leaves the player's data folder (%APPDATA%\nettai: builds, replays, settings, the extracted packs, the downloaded
# content) as it is.
#
# -Arch      x64 or arm64 (above): this machine's own, which is the default
# -Version   the version, if set; else $env:NETTAI_VERSION, else the app crate's
# -Icon      the .ico packaging/icons.sh makes, for the program and the installer; without it neither has one
# -Ffmpeg    the ffmpeg.exe to bundle (for the architecture packaged); else $env:NETTAI_BUNDLE_FFMPEG, else the release
#            packaging\ffmpeg.sha256 pins, downloaded and checked (a static program that needs no DLL)
# $env:WINDOWS_CERTIFICATE_PATH, $env:WINDOWS_CERTIFICATE_PASSWORD
#            a code-signing certificate (.pfx) vpk signs every program of the release with (the app's, ffmpeg,
#            Velopack's installer and updater); else none is signed
# $env:VPK   Velopack's vpk (the version crates/nettai's velopack is: 1.3.39-g8d2844f); else vpk on the PATH
param(
    [ValidateSet('x64', 'arm64')]
    [string]$Arch,
    [string]$Version = $env:NETTAI_VERSION,
    [string]$Icon,
    [string]$Ffmpeg = $env:NETTAI_BUNDLE_FFMPEG
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# The machine's architecture (the system's, whatever this PowerShell runs as), which the package's must be.
$machine = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    'X64' { 'x64' }
    'Arm64' { 'arm64' }
    default { throw "nettai is packaged on x64 or arm64 Windows, not $_" }
}
if (-not $Arch) { $Arch = $machine }
if ($Arch -ne $machine) { throw "the $Arch package is built on $Arch Windows, not on this $machine machine" }
# The architecture's channel, which is also Velopack's runtime; the bridges, the channels the same app goes up on
# besides; and the ffmpeg release's program for the architecture.
$package = @{
    'x64' = @{ Channel = 'win-x64'; Bridges = @('win'); Ffmpeg = 'ffmpeg-windows-x86_64.exe' }
    'arm64' = @{ Channel = 'win-arm64'; Bridges = @(); Ffmpeg = 'ffmpeg-windows-arm64.exe' }
}[$Arch]

$root = (Resolve-Path "$PSScriptRoot\..\..").Path
if (-not $Version) {
    $Version = (Select-String -Path "$root\Cargo.toml" -Pattern '^version = "(.*)"$' | Select-Object -First 1).Matches[0].Groups[1].Value
}
$out = "$root\target\package\windows\$Arch"
$app = "$out\nettai"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $app | Out-Null

# The release build, for a packaged app (`NETTAI_PACKAGED`: it plays the games' content it downloads into its
# store, none of the repository's), with
# the icon. Cargo's messages name the program and build.rs's OUT_DIR, where it wrote the third-party notices.
$env:NETTAI_PACKAGED = '1'
if ($Icon) { $env:NETTAI_WINDOWS_ICON = (Resolve-Path $Icon).Path }
$messages = & cargo build --release --locked -p nettai --message-format=json-render-diagnostics --manifest-path "$root\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
Remove-Item Env:NETTAI_PACKAGED
if ($Icon) { Remove-Item Env:NETTAI_WINDOWS_ICON }
$program = $null
$notices = $null
foreach ($line in $messages) {
    if (-not $line.StartsWith('{')) { continue }
    $message = $line | ConvertFrom-Json
    if ($message.reason -eq 'compiler-artifact' -and $message.executable) { $program = $message.executable }
    if ($message.reason -eq 'build-script-executed' -and (Test-Path "$($message.out_dir)\licenses.txt")) { $notices = "$($message.out_dir)\licenses.txt" }
}
if (-not $program) { throw "the build named no program" }
if (-not $notices) { throw "the build named no licenses.txt (crates/nettai/build.rs)" }

# A file of the ffmpeg release packaging\ffmpeg.sha256 pins (nettaisen/ffmpeg-build's, a minimal static
# build of what the replay export runs), downloaded once into target\package\ffmpeg\<tag>\ and checked: its path.
function Get-FfmpegFile([string]$file) {
    $pins = Get-Content "$root\packaging\ffmpeg.sha256"
    $release = ($pins | Select-String -Pattern '^# release: (.*)$' | Select-Object -First 1).Matches[0].Groups[1].Value
    $sum = $pins | ForEach-Object { if ($_ -match "^([0-9a-f]{64})  $([regex]::Escape($file))$") { $Matches[1] } } | Select-Object -First 1
    if (-not $sum) { throw "packaging\ffmpeg.sha256 pins no $file" }
    $dir = "$root\target\package\ffmpeg\$release"
    New-Item -ItemType Directory -Force $dir | Out-Null
    $path = "$dir\$file"
    if (-not (Test-Path $path) -or (Get-FileHash -Algorithm SHA256 $path).Hash.ToLower() -ne $sum) {
        Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/nettaisen/ffmpeg-build/releases/download/$release/$file" -OutFile $path
        if ((Get-FileHash -Algorithm SHA256 $path).Hash.ToLower() -ne $sum) {
            Remove-Item $path
            throw "$file of $release isn't the one pinned"
        }
    }
    return $path
}

Copy-Item $program "$app\nettai.exe"
# ffmpeg beside the program, where the export looks first (nettai_video::mp4::find_ffmpeg): the release's program for
# the architecture; and its notice (GPL: what it is built from, where that source is).
if (-not $Ffmpeg) { $Ffmpeg = Get-FfmpegFile $package.Ffmpeg }
Copy-Item $Ffmpeg "$app\ffmpeg.exe"
Copy-Item "$root\LICENSE" "$app\LICENSE.txt"
Copy-Item $notices "$app\THIRD-PARTY-NOTICES.txt"
Copy-Item (Get-FfmpegFile 'ffmpeg-NOTICE.txt') "$app\ffmpeg-NOTICE.txt"

# The release (Velopack's: docs/app.md §19), on the architecture's channel and on the bridge's, every program in it
# signed where there is a certificate.
$vpk = if ($env:VPK) { $env:VPK } else { 'vpk' }
foreach ($on in @($package.Channel) + $package.Bridges) {
    $pack = @('pack', '--packId', 'nettai', '--packVersion', $Version, '--packTitle', 'nettai', '--packAuthors', 'nettai',
        '--packDir', $app, '--mainExe', 'nettai.exe', '--channel', $on, '--runtime', $package.Channel, '--outputDir', "$out\releases")
    if ($Icon) { $pack += @('--icon', (Resolve-Path $Icon).Path) }
    if ($env:WINDOWS_CERTIFICATE_PATH) {
        $pack += @('--signParams', "/f `"$env:WINDOWS_CERTIFICATE_PATH`" /p `"$env:WINDOWS_CERTIFICATE_PASSWORD`" /fd SHA256 /tr http://timestamp.digicert.com /td SHA256")
    }
    & $vpk @pack
    if ($LASTEXITCODE -ne 0) { throw "vpk pack ($on) failed" }
}

Write-Output "$out\releases"
