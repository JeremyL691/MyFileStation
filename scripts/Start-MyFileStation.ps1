$ErrorActionPreference = 'Stop'
$runtimeId = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
$runtimeKeys = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$runtimeId",
    "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$runtimeId"
)
$installed = $false
foreach ($key in $runtimeKeys) {
    $value = (Get-ItemProperty -LiteralPath $key -Name pv -ErrorAction SilentlyContinue).pv
    $version = [version]::MinValue
    if ($value -and [version]::TryParse([string]$value, [ref]$version) -and $version -gt [version]'0.0.0.0') {
        $installed = $true
        break
    }
}

if (-not $installed) {
    Write-Host 'Microsoft Edge WebView2 Runtime is required to run MyFileStation.' -ForegroundColor Yellow
    Write-Host 'Opening the official Microsoft download page.'
    Start-Process 'https://developer.microsoft.com/microsoft-edge/webview2/'
    exit 2
}

$application = Join-Path $PSScriptRoot 'MyFileStation.exe'
if (-not (Test-Path -LiteralPath $application)) {
    throw "MyFileStation.exe was not found next to the launcher: $application"
}
Start-Process -FilePath $application -WorkingDirectory $PSScriptRoot
