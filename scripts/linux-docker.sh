#!/usr/bin/env bash
# Build e test Linux in un container (Docker su Colima, gratuito e locale).
#
# Uso:  scripts/linux-docker.sh [arm64|amd64]     (predefinito: arm64)
#
# - Il repository è montato in SOLA LETTURA; si compila una copia del commit
#   corrente (git archive) dentro il container.
# - Unica scrittura sul Mac: la cartella linux-out/<arch>/ del progetto
#   (pacchetto .deb e log; niente AppImage, vedi docs/MULTIPIATTAFORMA.md), ignorata da git.
# - amd64 su Apple Silicon usa Rosetta di Colima (rosetta: true).
# Prerequisito: Colima avviato dall'utente (`colima start`).
set -euo pipefail

arch="${1:-arm64}"
# Digest di ubuntu:24.04 per architettura, dall'indice ufficiale
# sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac.
case "$arch" in
  arm64) base="ubuntu@sha256:18b33ff3829707f31ada654a963b9ee35f2e6f0241b964bd17a22a9b330cb63d" ;;
  amd64) base="ubuntu@sha256:a2d43bffc46670d7108bb3c0468c67c5557c4c92e83e1e34cbff1322ad1b94dc" ;;
  *) echo "architettura non valida: $arch" >&2; exit 2 ;;
esac

root="$(cd "$(dirname "$0")/.." && pwd)"
out="$root/linux-out/$arch"
image="pezzaliapp-zipunzip-linux:$arch"
mkdir -p "$out"

docker build --platform "linux/$arch" --build-arg "BASE=$base" -t "$image" "$root/ci/linux"
got="$(docker image inspect "$image" --format '{{.Architecture}}')"
[ "$got" = "$arch" ] || { echo "immagine $image con architettura $got invece di $arch" >&2; exit 1; }

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
    npm run tauri build -- --bundles deb
    bin=src-tauri/target/release/pezzaliapp-zipunzip
    xvfb-run -a "$bin" --self-test | tee /out/self-test.txt
    find src-tauri/target/release/bundle -maxdepth 2 -name "*.deb" -exec cp {} /out/ \;
    (cd /out && sha256sum *.deb > SHA256SUMS.txt && cat SHA256SUMS.txt)
  '
echo "✓ risultati in $out"
