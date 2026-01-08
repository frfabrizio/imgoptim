$ErrorActionPreference = "Stop"

Write-Host "== imgoptim release checks =="

Write-Host "-> cargo test --tests"
cargo test --tests

# Run WebP tests if feature exists.
$hasWebp = Select-String -Path "Cargo.toml" -Pattern "^\s*webp\s*=" -Quiet
if ($hasWebp) {
    Write-Host "-> cargo test --tests --features webp"
    cargo test --tests --features webp
}

Write-Host "-> verify no libwebp-sys in Cargo.lock"
$hasLibwebp = Select-String -Path "Cargo.lock" -Pattern "libwebp-sys" -Quiet
if ($hasLibwebp) {
    throw "Cargo.lock contains libwebp-sys (not 100% Rust)."
}

Write-Host "-> verify no webp-sys in Cargo.lock"
$hasWebpSys = Select-String -Path "Cargo.lock" -Pattern "webp-sys" -Quiet
if ($hasWebpSys) {
    throw "Cargo.lock contains webp-sys (not 100% Rust)."
}

Write-Host "All checks passed."
