$ErrorActionPreference = "Stop"

if (-not $env:SOTOR_ASSETS_ZIP) {
    throw "Set SOTOR_ASSETS_ZIP to a generated SotOR assets.zip before building."
}

cargo build --release
Copy-Item "target\release\sotor.exe" "SotOR-Accessible.exe" -Force
Write-Host "Built SotOR-Accessible.exe"
