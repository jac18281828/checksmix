#!/usr/bin/env bash
# Seeds `assemble` and `mmo_decode`'s corpora from the repo's own example
# and soundness programs, split into pieces of at most 4 KB so
# `-max_len=4096` never discards a seed whole. `assemble`'s pieces stay
# line-aligned; a corpus item need not itself assemble.
set -euo pipefail

MAX_SEED_BYTES=4096

split_lines() {
  local src="$1" outdir="$2" base
  base=$(basename "$src")
  awk -v outdir="$outdir" -v base="$base" -v max="$MAX_SEED_BYTES" '
    BEGIN { n = 0; buf = ""; size = 0 }
    {
      line = $0 "\n"
      if (size > 0 && size + length(line) > max) {
        printf "%s", buf > (outdir "/" base "." n)
        close(outdir "/" base "." n)
        n++
        buf = ""
        size = 0
      }
      buf = buf line
      size += length(line)
    }
    END {
      if (size > 0) {
        printf "%s", buf > (outdir "/" base "." n)
      }
    }
  ' "$src"
}

split_bytes() {
  local src="$1" outdir="$2" base
  base=$(basename "$src")
  split -b "$MAX_SEED_BYTES" -d -a 4 "$src" "$outdir/$base."
}

seed_assemble() {
  local outdir="$1"
  mkdir -p "$outdir"
  for src in examples/*.mms tests/resources/soundness/*.mms; do
    if [ "$(wc -c <"$src")" -gt "$MAX_SEED_BYTES" ]; then
      split_lines "$src" "$outdir"
    else
      cp "$src" "$outdir/"
    fi
  done
}

seed_mmo_decode() {
  local mmixasm="$1" outdir="$2" src name mmo
  mkdir -p "$outdir"
  for src in examples/*.mms tests/resources/soundness/*.mms; do
    name=$(basename "$src" .mms)
    mmo="$outdir/$name.mmo.tmp"
    "$mmixasm" "$src" -o "$mmo"
    if [ "$(wc -c <"$mmo")" -gt "$MAX_SEED_BYTES" ]; then
      split_bytes "$mmo" "$outdir"
      rm -f "$mmo"
    else
      mv "$mmo" "$outdir/$name.mmo"
    fi
  done
}

case "${1:-}" in
  assemble)
    seed_assemble "$2"
    ;;
  mmo_decode)
    seed_mmo_decode "$2" "$3"
    ;;
  *)
    echo "usage: seed_corpus.sh assemble OUTDIR | seed_corpus.sh mmo_decode MMIXASM OUTDIR" >&2
    exit 2
    ;;
esac
