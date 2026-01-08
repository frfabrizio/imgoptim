## Notes

## Changes
- Conversion inter-formats JPEG/PNG/WebP via pipeline raw, avec background JPEG.
- Resize et cible de taille `--size` en mode convert (JPEG).
- Metadonnees: conservation par defaut, strip detaille, tagging XMP, support WebP XMP.
- WebP 100% Rust (pas de bindings C).
- Tests enrichis et reorganises (`tests/` + assets).

## Checks
- [ ] `scripts/release_checks.ps1`
- [ ] `cargo build --release`
