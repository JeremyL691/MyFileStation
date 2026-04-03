$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path

Set-Location $projectRoot
python -m pip install -r requirements-packaging.txt
python -m PyInstaller --noconfirm --clean MyFileStation.spec
