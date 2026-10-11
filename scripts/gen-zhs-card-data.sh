#!/usr/bin/env bash
set -euo pipefail

# Generate the Simplified-Chinese card-content sidecar (`card-data.zh.json`).
#
# Sibling of the MTGJSON-derived sidecars that `oracle-gen --sidecar-dir` writes.
# Those cover es/fr/de/it/ja/pt from MTGJSON `foreignData`; Chinese comes from a
# different source (see scripts/lib/magic_cards_zhs.py for why), so it needs its
# own generator rather than another entry in oracle-gen's locale map.
#
# Source: https://github.com/HeliumOctahelide/magic-cards-zhs
# Licensed CC-BY-SA-4.0. See NOTICE and DMCA.md for the attribution and the
# Wizards-of-the-Coast provenance caveat that source does not itself address.
#
# The upstream export is a ~78 MB tar.gz republished every Saturday, containing
# ~381 MB of JSONL. Only `zhs_oracle.json` is needed, so the archive is cached
# and that one member is streamed out of it rather than extracting all seven
# files.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Pinned deliberately. Upstream rotates old releases out of the releases API
# (only 3 of its 44 tags still carry assets), so "always resolve latest" would
# make this generator fail on a week when the newest release is briefly absent,
# and would silently change the committed data. Bump this tag on a data refresh.
RELEASE_TAG="${MAGIC_CARDS_ZHS_RELEASE:-data-2026-10-11}"
# Upstream's asset name repeats the `data-` prefix inside the filename:
# tag `data-2026-10-11` -> asset `magic-cards-zhs-data-2026-10-11.tar.gz`.
ASSET_NAME="magic-cards-zhs-${RELEASE_TAG}.tar.gz"
ARCHIVE_URL="${MAGIC_CARDS_ZHS_URL:-https://github.com/HeliumOctahelide/magic-cards-zhs/releases/download/$RELEASE_TAG/$ASSET_NAME}"

CACHE_DIR="${MAGIC_CARDS_ZHS_CACHE_DIR:-data/magic-cards-zhs}"
ARCHIVE="$CACHE_DIR/$ASSET_NAME"
SOURCE_MEMBER="zhs_oracle.json"
EXTRACTED="$CACHE_DIR/$SOURCE_MEMBER"
OUTPUT="${MAGIC_CARDS_ZHS_OUTPUT:-client/public/card-data.zh.json}"

echo "=== Simplified Chinese Card Data Generation ($RELEASE_TAG) ==="

mkdir -p "$CACHE_DIR"

if [ ! -f "$EXTRACTED" ]; then
  if [ ! -f "$ARCHIVE" ]; then
    echo "Downloading $ASSET_NAME..."
    # --http1.1: GitHub's HTTP/2 edge aborts this particular asset mid-transfer
    # with PROTOCOL_ERROR often enough to matter; --retry-all-errors covers the
    # remaining resets. The download is resumable, so a partial file is kept.
    curl --fail --http1.1 --retry 5 --retry-all-errors --retry-delay 3 \
      --connect-timeout 30 -L -C - \
      -H 'User-Agent: phase-rs-card-data/1.0 (+https://github.com/phase-rs/phase)' \
      -o "$ARCHIVE" "$ARCHIVE_URL"
    echo "  downloaded $(du -h "$ARCHIVE" | cut -f1)"
  fi

  # Fail loudly on a truncated or corrupt archive rather than letting tar emit a
  # partial JSONL that the builder would then read as a short-but-valid export.
  if ! gzip -t "$ARCHIVE"; then
    echo "ERROR: $ARCHIVE is not a valid gzip archive (truncated download?)." >&2
    echo "  Delete it and re-run to retry." >&2
    exit 1
  fi

  echo "Extracting $SOURCE_MEMBER..."
  tar -xzf "$ARCHIVE" -C "$CACHE_DIR" "./$SOURCE_MEMBER"
fi

if [ ! -s "$EXTRACTED" ]; then
  echo "ERROR: $EXTRACTED is missing or empty." >&2
  exit 1
fi

echo "Building $OUTPUT..."
mkdir -p "$(dirname "$OUTPUT")"
python3 "$SCRIPT_DIR/lib/magic_cards_zhs.py" --source "$EXTRACTED" --output "$OUTPUT.tmp"

# Promote only after the builder has written a complete file, mirroring
# oracle-gen's tmp+rename so a reader never sees a partial sidecar.
mv -f "$OUTPUT.tmp" "$OUTPUT"

echo "Generated $OUTPUT ($(du -h "$OUTPUT" | cut -f1))"
