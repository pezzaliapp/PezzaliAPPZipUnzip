# Ambiente di build e test Linux per PezzaliAPPZipUnzip (Ubuntu 24.04).
# Usato da scripts/linux-docker.sh; non contiene sorgenti né credenziali.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl wget file git xz-utils pkg-config \
      libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
      xvfb xauth zip unzip \
    && rm -rf /var/lib/apt/lists/*

# Node.js dai binari ufficiali, con verifica SHA-256 dal file SHASUMS256.txt.
ARG NODE_VERSION=24.19.0
RUN set -eu; \
    case "$(dpkg --print-architecture)" in arm64) a=arm64 ;; amd64) a=x64 ;; *) exit 1 ;; esac; \
    f="node-v${NODE_VERSION}-linux-${a}.tar.xz"; \
    cd /tmp; \
    curl -fsSLO "https://nodejs.org/dist/v${NODE_VERSION}/${f}"; \
    curl -fsSLO "https://nodejs.org/dist/v${NODE_VERSION}/SHASUMS256.txt"; \
    grep " ${f}\$" SHASUMS256.txt | sha256sum -c -; \
    tar -xJf "$f" -C /usr/local --strip-components=1; \
    rm -f "$f" SHASUMS256.txt; \
    node --version

# Rust (toolchain fissata, profilo minimo) installato solo nel container.
ARG RUST_VERSION=1.96.0
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal \
      --default-toolchain "${RUST_VERSION}" --component clippy,rustfmt \
    && rustc --version

# AppImage senza FUSE dentro il container.
ENV APPIMAGE_EXTRACT_AND_RUN=1
WORKDIR /work
