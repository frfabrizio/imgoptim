# imgoptim

Optimiseur et convertisseur d’images multi-format (JPEG/PNG/WebP) inspiré de
`jpegoptim`. Le binaire `imgoptim` vise un usage simple en ligne de commande
pour optimiser, convertir et uniformiser des lots d’images.

## Fonctionnalités

- Optimisation JPEG/PNG/WebP avec modes lossless/lossy selon les options.
- Conversion entre formats via `--output-format`.
- Contrôle fin de la qualité, de la taille cible et des métadonnées.
- Politique de nommage flexible (suffixe, in-place, destination dédiée).
- Mode dry-run, seuil minimal de gain et statistiques finales.

## Installation

Prérequis : Rust (édition 2021).

```bash
cargo build --release
```

Le binaire est disponible dans `target/release/imgoptim`.

## Utilisation rapide

### Optimiser des images (même format en sortie)

```bash
imgoptim --name-suffix "_imgoptim" photos/*.jpg
```

### Convertir vers un autre format

```bash
imgoptim --output-format webp --name-suffix "_imgoptim" images/*.png
```

### Ajouter un tag et nettoyer les métadonnées

```bash
imgoptim --tag-category "Optimisé" --strip-exif photos/*.jpg
```

## Options principales

### Options globales

- `--dest <path>` : dossier de sortie.
- `--overwrite` : écraser le fichier cible si déjà existant.
- `--preserve` : préserver les timestamps.
- `--noaction` : dry-run (n’écrit aucun fichier).
- `--threshold <percent>` : gain minimal (%) pour remplacer la cible.
- `--size <size>` : taille cible en KB ou % (active le mode lossy).
- `--totals` : résumé global après traitement.
- `--quiet` / `--verbose` : niveau de logs.
- `--output-format <fmt>` : conversion sans sous-commande (jpeg/png/webp).

### Nommage

- `--name-suffix <s>` : suffixe avant extension.
- `--keep-ext` : conserve l’extension d’origine (utile en conversion).
- `--inplace` : remplace l’extension dans la destination.

### Filtrage de formats

- `--only <fmt>` : ne traiter que ces formats.
- `--skip <fmt>` : ignorer ces formats.

### Métadonnées

- `--strip-all`, `--strip-exif`, `--strip-xmp`, `--strip-iptc`,
  `--strip-icc`, `--strip-com`
- `--keep-metadata` : ignore les options de stripping.
- `--tag-category <text>` : ajoute un tag métier si supporté.

### Qualité / formats

- `--max <quality>` : qualité max JPEG (lossy).
- `--quality <q>` : qualité générique (mapping par format).
- `--jpeg-turbo` : privilégier libjpeg-turbo si dispo.
- `--png-level <n>` / `--png-zopfli`
- `--webp-lossless` : force l’encodage lossless WebP.

### Conversion

```bash
imgoptim --output-format <fmt> [options] <files...>
```

Options spécifiques :

- `--output <fmt>` : format cible (jpeg/png/webp).
- `--input <fmt>` : n’accepter que ce format en entrée.
- `--lossless` / `--lossy` : stratégie de conversion.
- `--background <hex>` : fond utilisé pour la conversion vers JPEG.
- `--resize <spec>` : redimensionnement (WxH, Wx, xH).
- `--fit <mode>` : `contain`, `cover`, `stretch`.

## Exemples supplémentaires

```bash
# Optimiser uniquement les PNG
imgoptim --only png assets/*.*

# Forcer un seuil minimal de 5%
imgoptim --threshold 5 photos/*.jpg

# Convertir en JPEG en redimensionnant
imgoptim --output-format jpeg --resize 1200x --fit contain images/*.png
```
