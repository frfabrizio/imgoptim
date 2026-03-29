#!/usr/bin/env bash
set -euo pipefail

# Synchronise la branche courante avec origin/main en conservant l'historique local.
# Usage:
#   scripts/sync_with_main.sh            # rebase sur origin/main
#   scripts/sync_with_main.sh --merge    # merge origin/main

MODE="rebase"
if [[ "${1:-}" == "--merge" ]]; then
  MODE="merge"
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  echo "Erreur: ce dossier n'est pas un dépôt git." >&2
  exit 1
fi

CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$CURRENT_BRANCH" == "HEAD" ]]; then
  echo "Erreur: HEAD détachée. Checkout une branche avant sync." >&2
  exit 1
fi

if ! git remote get-url origin >/dev/null 2>&1; then
  echo "Erreur: remote 'origin' absent." >&2
  echo "Ajoute un remote, puis relance (ex: git remote add origin <url>)." >&2
  exit 2
fi

# Création de main local si absent (utile pour les scripts CI/outils).
if ! git show-ref --verify --quiet refs/heads/main; then
  git branch main "$CURRENT_BRANCH"
fi

git fetch origin main

if [[ "$MODE" == "merge" ]]; then
  git merge --no-ff origin/main
else
  git rebase origin/main
fi

echo "OK: branche '$CURRENT_BRANCH' synchronisée avec origin/main ($MODE)."
