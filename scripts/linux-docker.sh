#!/usr/bin/env bash
# Build e test Linux in un container (Docker su Colima, gratuito e locale).
#
# Uso:  scripts/linux-docker.sh [arm64|amd64]     (predefinito: arm64)
#
# - Il repository è montato in SOLA LETTURA; si compila una copia del commit
#   corrente (git archive) dentro il container.
# - Unica scrittura sul Mac: la cartella linux-out/<arch>/ del progetto
#   (pacchetti .deb/.AppImage e log), ignorata da git.
# - amd64 su Apple Silicon usa Rosetta di Colima (rosetta: true).
# Prerequisito: Colima avviato dall'utente (`colima start`).
set -euo pipefail

arch="${1:-arm64}"
case "$arch" in arm64|amd64) ;; *) echo "architettura non valida: $arch" >&2; exit 2 ;; esac

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/linux-out/$arch"
image="pezzaliapp-zipunzip-linux:$arch"
mkdir -p "$out"

docker build --platform "linux/$arch" -t "$image" "$root/ci/linux"

docker run --rm --platform "linux/$arch" \
  -v "$root:/repo:ro" -v "$out:/out" \
  "$image" bash -euo pipefail -c '
    exec > >(tee /out/log.txt) 2>&1
    git config --global --add safe.directory /repo
    git -C /repo archive HEAD | tar -x -C /work
    cd /work
    uname -a; node --version; rustc --version
    npm ci
    npm run fetch-7zip
    npx tsc --noEmit
    npm test
    (cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings)
    npm run tauri build -- --bundles deb,appimage
    bin=src-tauri/target/release/pezzaliapp-zipunzip
    xvfb-run -a "$bin" --self-test | tee /out/self-test.txt
    find src-tauri/target/release/bundle -maxdepth 2 \( -name "*.deb" -o -name "*.AppImage" \) -exec cp {} /out/ \;
    (cd /out && sha256sum *.deb *.AppImage > SHA256SUMS.txt && cat SHA256SUMS.txt)
  '
echo "✓ risultati in $out"
