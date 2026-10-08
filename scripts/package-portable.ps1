$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$releaseRoot = Join-Path $repositoryRoot 'release'
$stageRoot = Join-Path $releaseRoot 'portable-stage'
$appConfig = Get-Content -LiteralPath (Join-Path $repositoryRoot 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$zipPath = Join-Path $releaseRoot ('MyFileStation-{0}-win-x64-portable.zip' -f $appConfig.version)
$exeCandidates = @(
    (Join-Path $repositoryRoot 'target\release\myfilestation.exe'),
    (Join-Path $repositoryRoot 'target\release\MyFileStation.exe')
)
$applicationExe = $exeCandidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1

if (-not $applicationExe) {
    throw 'Release executable not found. Build the Tauri application first with scripts/build-windows.ps1.'
}

$resourceRoot = Join-Path $repositoryRoot 'target\release\resources'
if (-not (Test-Path -LiteralPath (Join-Path $resourceRoot 'drag-icon.png') -PathType Leaf)) {
    throw 'The drag icon resource is missing. Rebuild the Tauri application before packaging.'
}

New-Item -ItemType Directory -Path $releaseRoot -Force | Out-Null
$releaseFullPath = [IO.Path]::GetFullPath($releaseRoot).TrimEnd('\') + '\'
$stageFullPath = [IO.Path]::GetFullPath($stageRoot)
$zipFullPath = [IO.Path]::GetFullPath($zipPath)
if (-not $stageFullPath.StartsWith($releaseFullPath, [StringComparison]::OrdinalIgnoreCase) -or
    -not $zipFullPath.StartsWith($releaseFullPath, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Portable package output escaped the release directory.'
}

if (Test-Path -LiteralPath $stageFullPath) {
    Remove-Item -LiteralPath $stageFullPath -Recurse -Force
}
if (Test-Path -LiteralPath $zipFullPath) {
    Remove-Item -LiteralPath $zipFullPath -Force
}
New-Item -ItemType Directory -Path $stageFullPath -Force | Out-Null
Copy-Item -LiteralPath $applicationExe -Destination (Join-Path $stageFullPath 'MyFileStation.exe')

if (Test-Path -LiteralPath $resourceRoot) {
    Copy-Item -LiteralPath $resourceRoot -Destination $stageFullPath -Recurse -Force
}

Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $stageFullPath
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Start-MyFileStation.ps1') -Destination $stageFullPath
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Start-MyFileStation.cmd') -Destination $stageFullPath
@'
MyFileStation portable edition

Run Start-MyFileStation.cmd. The launcher checks the documented WebView2 Runtime registration keys and opens Microsoft's download page when the runtime is missing.
Application data is stored in %LOCALAPPDATA%\MyFileStation\v2.
'@ | Set-Content -LiteralPath (Join-Path $stageFullPath 'README-PORTABLE.txt') -Encoding utf8

Compress-Archive -Path (Join-Path $stageFullPath '*') -DestinationPath $zipFullPath -CompressionLevel Optimal
Remove-Item -LiteralPath $stageFullPath -Recurse -Force

$artifacts = @(Get-ChildItem -LiteralPath $releaseRoot -File | Where-Object { $_.Extension -in '.exe', '.zip' })
$hashLines = $artifacts | ForEach-Object {
    $hash = Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
    '{0}  {1}' -f $hash.Hash.ToLowerInvariant(), $_.Name
}
$hashLines | Set-Content -LiteralPath (Join-Path $releaseRoot 'SHA256SUMS.txt') -Encoding utf8
Write-Host "Created $zipPath"
