# Verify the Kairo-for-Zed integration.
# Run from the repository root on a machine with Rust 1.98.1 installed.
$ErrorActionPreference = 'Stop'

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "cargo not found. Install the Rust toolchain pinned in rust-toolchain.toml (1.98.1)."
}

Write-Host "== Rust crates =="
cargo check -p kairo_core -p kairo_ui -p zed
if ($LASTEXITCODE -ne 0) { throw "cargo check failed" }

Write-Host "== WASM extension =="
cargo build -p kairo_java --target wasm32-wasip2
if ($LASTEXITCODE -ne 0) { throw "extension build failed" }

Write-Host "== Clippy =="
./script/clippy

Write-Host "All Kairo integration checks passed."
