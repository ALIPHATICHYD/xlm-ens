#!/usr/bin/env bash
# Detect unexpected snapshot drift after tests run.
#
# Usage:
#   scripts/check-snapshots.sh [snapshot-dir]
#
# The check fails if tracked snapshot files were modified or deleted, or if new
# snapshot files were created without being committed.

set -euo pipefail

SNAPSHOT_DIR="${1:-tests/test_snapshots}"

if [[ ! -d "$SNAPSHOT_DIR" ]]; then
  echo "error: snapshot directory '$SNAPSHOT_DIR' does not exist." >&2
  exit 2
fi

mapfile -t TRACKED_CHANGES < <(git diff --name-only -- "$SNAPSHOT_DIR" || true)
mapfile -t UNTRACKED_CHANGES < <(git ls-files --others --exclude-standard -- "$SNAPSHOT_DIR" || true)

if [[ ${#TRACKED_CHANGES[@]} -eq 0 && ${#UNTRACKED_CHANGES[@]} -eq 0 ]]; then
  echo "Snapshot files are clean: $SNAPSHOT_DIR"
  exit 0
fi

echo "::error title=Snapshot review needed::Snapshot files changed after tests. Review the diff and commit intentional updates."
echo "Changed snapshot files:"

if [[ ${#TRACKED_CHANGES[@]} -gt 0 ]]; then
  printf '  %s\n' "${TRACKED_CHANGES[@]}"
fi

if [[ ${#UNTRACKED_CHANGES[@]} -gt 0 ]]; then
  printf '  %s\n' "${UNTRACKED_CHANGES[@]}"
fi

echo ""
echo "Git diff for tracked snapshot changes:"
if [[ ${#TRACKED_CHANGES[@]} -gt 0 ]]; then
  git diff -- "$SNAPSHOT_DIR"
else
  echo "  (none)"
fi

if [[ ${#UNTRACKED_CHANGES[@]} -gt 0 ]]; then
  echo ""
  echo "Diff for new snapshot files:"
  for file in "${UNTRACKED_CHANGES[@]}"; do
    echo ""
    echo "--- $file"
    git diff --no-index -- /dev/null "$file" || true
  done
fi

exit 1
