#!/usr/bin/env pwsh
$ErrorActionPreference = "Stop"

# GitHub requires TLS 1.2
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = "farrelaby/dirsweep"
$Version = if ($env:VERSION) { $env:VERSION } else { "latest" }
$InstallDir = if ($env:INSTALL_DIR) { $env:INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }

$RawArch = $env:PROCESSOR_ARCHITECTURE
if (-not $RawArch) { $RawArch = "AMD64" }
$Arch = switch ($RawArch) {
  "AMD64" { "x86_64" }
  "ARM64" { "aarch64" }
  "x86" { "x86_64" }
  default { throw "Unsupported architecture: $RawArch" }
}
$Target = "$Arch-pc-windows-msvc"

if ($Version -eq "latest") {
  $api = Invoke-RestMethod -UseBasicParsing "https://api.github.com/repos/$Repo/releases/latest"
  $Version = $api.tag_name -replace '^v', ''
  if (-not $Version) { throw "Failed to fetch latest version from GitHub" }
}

$ArchiveName = "dirsweep-v$Version-$Target.zip"
$Url = "https://github.com/$Repo/releases/download/v$Version/$ArchiveName"
$TmpDir = "$env:TEMP\dirsweep-install"
$TmpZip = "$TmpDir\dirsweep.zip"

Write-Host "Downloading dirsweep v$Version for $Target..."
New-Item -ItemType Directory -Force -Path $TmpDir | Out-Null

try {
  Invoke-WebRequest -UseBasicParsing -Uri $Url -OutFile $TmpZip
} catch {
  Write-Host "Download failed: $_"
  exit 1
}

Write-Host "Extracting..."
try {
  Expand-Archive -Path $TmpZip -DestinationPath $TmpDir -Force
} catch {
  Add-Type -Assembly System.IO.Compression.FileSystem
  $zip = [System.IO.Compression.ZipFile]::OpenRead($TmpZip)
  $zip.Entries | ForEach-Object {
    $dest = Join-Path $TmpDir $_.FullName
    $dir = [System.IO.Path]::GetDirectoryName($dest)
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
    [System.IO.Compression.ZipFileExtensions]::ExtractToFile($_, $dest, $true)
  }
  $zip.Dispose()
}

$Binary = "$TmpDir\dirsweep.exe"
if (-not (Test-Path $Binary)) {
  Write-Host "Error: Binary not found in archive"
  exit 1
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Move-Item -Path $Binary -Destination "$InstallDir\dirsweep.exe" -Force

Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue

$UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($UserPath -notlike "*$InstallDir*") {
  $NewPath = "$InstallDir;$UserPath"
  [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
  Write-Host "Added $InstallDir to user PATH"
}

Write-Host "dirsweep v$Version installed to $InstallDir\dirsweep.exe"
Write-Host "Restart your terminal or run: `$env:PATH = `"$InstallDir;`$env:PATH`""