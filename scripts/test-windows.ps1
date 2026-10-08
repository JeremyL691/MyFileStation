param([switch] $IncludeStress)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path -Parent $PSScriptRoot

function Invoke-QualityCheck([string] $Name, [scriptblock] $Action) {
    Write-Host "Running $Name"
    & $Action
    if ($LASTEXITCODE -ne 0) {
        throw "$Name failed with exit code $LASTEXITCODE."
    }
}

Push-Location $repositoryRoot
try {
    Invoke-QualityCheck 'locked dependency installation' { pnpm install --frozen-lockfile }
    Invoke-QualityCheck 'frontend tests' { pnpm test }
    Invoke-QualityCheck 'TypeScript' { pnpm typecheck }
    Invoke-QualityCheck 'ESLint' { pnpm lint }
    Invoke-QualityCheck 'Prettier' { pnpm format:check }
    Invoke-QualityCheck 'frontend production build' { pnpm --filter @myfilestation/frontend build:web }
    Invoke-QualityCheck 'Rust formatting' { cargo fmt --all --check }
    Invoke-QualityCheck 'Rust tests' { cargo test --workspace --locked }
    Invoke-QualityCheck 'Clippy' { cargo clippy --workspace --all-targets --locked -- -D warnings }
    if ($IncludeStress) {
        Invoke-QualityCheck 'storage stress' { cargo test --workspace --locked -- --ignored --nocapture }
    }
    Write-Host 'All requested quality checks passed.'
} finally {
    Pop-Location
}
