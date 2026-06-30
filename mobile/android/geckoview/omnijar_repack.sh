#!/bin/sh
# Repack a .ja (ZIP) stored uncompressed, compress with XZ, write SHA-256 sidecar.
# Usage: omnijar_repack.sh <src> <dst-file> <dst-tag>
set -e
src="$1" dst_file="$2" dst_tag="$3"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
cp "$src" "$tmp/omni.ja"
# Repack all entries as ZIP_STORED (strip per-entry deflate)
( cd "$tmp" && unzip -q omni.ja -d unpacked && cd unpacked && zip -q -0 -r ../repacked.ja . )
xz -9e -k -c "$tmp/repacked.ja" > "$dst_file"
sha256sum -b "$tmp/repacked.ja" | awk '{print $1}' > "$dst_tag"
