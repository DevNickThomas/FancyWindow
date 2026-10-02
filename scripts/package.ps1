# Builds the release and packs it for distribution:
#   dist\FancyWindow-<version>-win64.zip  (FancyWindow.exe, README, licences)
#   dist\FancyWindow-<version>-win64.zip.sha256
# Needs Rust and MinGW (C:\mingw64\bin) on PATH, as in the README.
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot)

$version = (Select-String -Path Cargo.toml -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$name = "FancyWindow-$version-win64"

cargo test --quiet
if ($LASTEXITCODE -ne 0) { throw 'tests failed' }
cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'build failed' }

$stage = Join-Path 'dist' $name
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory $stage | Out-Null
Copy-Item 'target\release\fancy-window.exe' (Join-Path $stage 'FancyWindow.exe')
Copy-Item 'README.md', 'THIRD-PARTY-NOTICES.txt' $stage
if (Test-Path 'LICENSE') { Copy-Item 'LICENSE' $stage } else { Write-Warning 'No LICENSE file: add one before publishing.' }

$zip = "dist\$name.zip"
Compress-Archive -Path "$stage\*" -DestinationPath $zip -Force
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
"$hash  $name.zip" | Set-Content "$zip.sha256" -Encoding ascii
Write-Output "$zip`n$hash"
