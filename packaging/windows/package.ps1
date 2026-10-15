# The Windows installer, and the same app in a .zip, on Windows:
# target\package\windows\nettai-<version>-windows-x64-setup.exe and nettai-<version>-windows-x64.zip.
#
#   pwsh packaging\windows\package.ps1 [-Version <version>] [-Icon <nettai.ico>] [-Ffmpeg <ffmpeg.exe>]
#
# The app: nettai.exe, ffmpeg.exe, content\, LICENSE.txt, THIRD-PARTY-NOTICES.txt, ffmpeg-NOTICE.txt. No ROM and no
# extracted asset is in it: the
# app extracts each game's pack from the player's own copy. The installer is Inno Setup's (nettai.iss): ISCC.exe
# on the PATH or in Inno Setup 6's folder (`winget install JRSoftware.InnoSetup` or `choco install innosetup`).
#
# -Version   the version (the workflow's, from the tag); else $env:NETTAI_VERSION, else the app crate's
# -Icon      the .ico packaging/icons.sh makes, for the program and the installer; without it neither has one
# -Ffmpeg    the ffmpeg.exe to bundle; else $env:NETTAI_BUNDLE_FFMPEG, else the release packaging\ffmpeg.sha256
#            pins, downloaded and checked (a static program that needs no DLL)
# $env:WINDOWS_CERTIFICATE_PATH, $env:WINDOWS_CERTIFICATE_PASSWORD
#            a code-signing certificate (.pfx) to sign the program and the installer with (signtool, from the
#            Windows SDK); else neither is signed
param(
    [string]$Version = $env:NETTAI_VERSION,
    [string]$Icon,
    [string]$Ffmpeg = $env:NETTAI_BUNDLE_FFMPEG
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path "$PSScriptRoot\..\..").Path
if (-not $Version) {
    $Version = (Select-String -Path "$root\crates\nettai\Cargo.toml" -Pattern '^version = "(.*)"$' | Select-Object -First 1).Matches[0].Groups[1].Value
}
$out = "$root\target\package\windows"
$app = "$out\nettai"
if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force $app | Out-Null

# The release build, for a packaged app (`NETTAI_PACKAGED`: it reads no content but the content it ships), with
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

$certificate = $env:WINDOWS_CERTIFICATE_PATH
function Sign-File([string]$file) {
    if (-not $certificate) { return }
    $signtool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" | Sort-Object FullName | Select-Object -Last 1
    & $signtool.FullName sign /f $certificate /p $env:WINDOWS_CERTIFICATE_PASSWORD /fd SHA256 /tr http://timestamp.digicert.com /td SHA256 $file
    if ($LASTEXITCODE -ne 0) { throw "signtool failed on $file" }
}

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
Sign-File "$app\nettai.exe"
# ffmpeg beside the program, where the export looks first (nettai_video::mp4::find_ffmpeg); and its notice (GPL: what
# it is built from, where that source is).
if (-not $Ffmpeg) { $Ffmpeg = Get-FfmpegFile 'ffmpeg-windows-x86_64.exe' }
Copy-Item $Ffmpeg "$app\ffmpeg.exe"
Sign-File "$app\ffmpeg.exe"
Copy-Item -Recurse "$root\content" "$app\content"
Copy-Item "$root\LICENSE" "$app\LICENSE.txt"
Copy-Item $notices "$app\THIRD-PARTY-NOTICES.txt"
Copy-Item (Get-FfmpegFile 'ffmpeg-NOTICE.txt') "$app\ffmpeg-NOTICE.txt"

Compress-Archive -Path $app -DestinationPath "$out\nettai-$Version-windows-x64.zip"

$iscc = (Get-Command iscc.exe -ErrorAction SilentlyContinue).Source
if (-not $iscc) {
    $iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") |
        Where-Object { Test-Path $_ } | Select-Object -First 1
}
if (-not $iscc) { throw "Inno Setup 6's ISCC.exe wasn't found (winget install JRSoftware.InnoSetup)" }
$defines = @("/DAppVersion=$Version", "/DSourceDir=$app", "/DOutputDir=$out")
if ($Icon) { $defines += "/DIconFile=$((Resolve-Path $Icon).Path)" }
& $iscc /Q @defines "$PSScriptRoot\nettai.iss"
if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }
$setup = "$out\nettai-$Version-windows-x64-setup.exe"
Sign-File $setup

Write-Output $setup
