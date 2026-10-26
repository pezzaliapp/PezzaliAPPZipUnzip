# Ambiente di build e test Linux per PezzaliAPPZipUnzip (Ubuntu 24.04).
# Usato da scripts/linux-docker.sh; non contiene sorgenti né credenziali.
# Immagine ufficiale Docker "library/ubuntu" 24.04, fissata per digest. L'indice
# multi-architettura sha256:68ee22004a91… è stato verificato sul registro il
# 25 ottobre 2026; scripts/linux-docker.sh passa il digest della singola
# architettura (il builder classico di Docker ignora --platform con un indice).
ARG BASE=ubuntu:24.04@sha256:68ee22004a9109b44973313b02ff7b1de250f1de0f5ecb628d73f579aaff33ac
FROM ${BASE}

ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y --no-install-recommends \
      build-essential ca-certificates curl wget file git xz-utils pkg-config \
      libwebkit2gtk-4.1-dev libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev \
      xvfb xauth zip unzip \
    && rm -rf /var/lib/apt/lists/*

# Node.js dai binari ufficiali. Hash presi da SHASUMS256.txt, la cui firma GPG
# (team di rilascio di Node.js) è stata verificata il 25 ottobre 2026.
ARG NODE_VERSION=24.19.0
RUN set -eu; \
    case "$(dpkg --print-architecture)" in \
      arm64) a=arm64; h=01443c1e1a29e531ccad5a46fefa6df490d2189c49f7955904aecdbb0fe86fdc ;; \
      amd64) a=x64;   h=14b342e71204f811bde6153be8e04b62aef63c236fef92b55f9c83154b409647 ;; \
      *) exit 1 ;; \
    esac; \
    f="node-v${NODE_VERSION}-linux-${a}.tar.xz"; \
    curl -fsSL -o "/tmp/$f" "https://nodejs.org/dist/v${NODE_VERSION}/${f}"; \
    echo "$h  /tmp/$f" | sha256sum -c -; \
    tar -xJf "/tmp/$f" -C /usr/local --strip-components=1; \
    rm -f "/tmp/$f"; \
    node --version

# Rust (toolchain fissata, profilo minimo) installato solo nel container con
# l'installatore ufficiale; rustup verifica l'hash di ogni componente scaricato.
ARG RUST_VERSION=1.96.0
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo PATH=/opt/cargo/bin:$PATH
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal \
      --default-toolchain "${RUST_VERSION}" --component clippy,rustfmt \
    && rustc --version

# Locale UTF-8 come su un desktop Linux: in locale POSIX `unzip` (Info-ZIP)
# non ricrea i nomi di file non ASCII (test d'interoperabilità).
ENV LANG=C.UTF-8
WORKDIR /work
