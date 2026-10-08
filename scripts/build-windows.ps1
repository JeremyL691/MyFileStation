param(
    [switch] $SkipInstaller
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot
$releaseRoot = Join-Path $repositoryRoot 'release'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'

if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Visual Studio Installer / vswhere.exe was not found. Install Visual Studio 2022 C++ Build Tools and the Windows SDK.'
}

$visualStudioRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $visualStudioRoot) {
    throw 'The Visual Studio C++ x64 toolchain was not found. Install the Visual Studio 2022 C++ Build Tools workload.'
}

$vsDevCmd = Join-Path $visualStudioRoot 'Common7\Tools\VsDevCmd.bat'
$initCommand = 'call "' + $vsDevCmd + '" -arch=x64 -host_arch=x64 -no_logo >nul && set'
$environmentLines = & $env:ComSpec /d /s /c $initCommand
foreach ($line in $environmentLines) {
    $separator = $line.IndexOf('=')
    if ($separator -gt 0) {
        [Environment]::SetEnvironmentVariable($line.Substring(0, $separator), $line.Substring($separator + 1), 'Process')
    }
}

Push-Location $repositoryRoot
try {
    pnpm install --frozen-lockfile
    if ($LASTEXITCODE -ne 0) { throw 'pnpm install failed.' }

    if ($SkipInstaller) {
        pnpm --filter @myfilestation/frontend build --no-bundle
    } else {
        pnpm build
    }
    if ($LASTEXITCODE -ne 0) { throw 'The Tauri build failed.' }

    New-Item -ItemType Directory -Path $releaseRoot -Force | Out-Null
    if (-not $SkipInstaller) {
        $installerRoot = Join-Path $repositoryRoot 'target\release\bundle\nsis'
        $installers = @(Get-ChildItem -LiteralPath $installerRoot -Filter '*.exe' -File -ErrorAction SilentlyContinue)
        if ($installers.Count -eq 0) { throw "Tauri completed without producing an NSIS installer in $installerRoot." }
        foreach ($installer in $installers) {
            Copy-Item -LiteralPath $installer.FullName -Destination $releaseRoot -Force
        }
    }

    $artifacts = @(Get-ChildItem -LiteralPath $releaseRoot -File | Where-Object { $_.Extension -in '.exe', '.zip' })
    if ($artifacts.Count -gt 0) {
        $hashLines = $artifacts | ForEach-Object {
            $hash = Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256
            '{0}  {1}' -f $hash.Hash.ToLowerInvariant(), $_.Name
        }
        $hashLines | Set-Content -LiteralPath (Join-Path $releaseRoot 'SHA256SUMS.txt') -Encoding utf8
    }

    Write-Host "Build artifacts are in $releaseRoot"
} finally {
    Pop-Location
}
