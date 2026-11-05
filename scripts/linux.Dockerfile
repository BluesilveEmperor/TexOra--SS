# Builds TeXora for Linux, then tests the package in a second clean system.
# Nothing here can touch the computer's files; only the last stage is handed back.
FROM rust:1-bookworm AS build
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    pkg-config libxcb1-dev libxcb-randr0-dev libxkbcommon-dev libwayland-dev >/dev/null
WORKDIR /src
COPY Cargo.toml Cargo.lock build.rs README.md CREDITS.md LICENSE ./
COPY src src
COPY assets assets
RUN cargo test --release --locked
RUN cargo build --release --locked
RUN set -e; \
    V=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1); \
    case "$(uname -m)" in x86_64) A=x64 ;; aarch64) A=arm64 ;; *) A=$(uname -m) ;; esac; \
    P="TeXora-$V-linux-$A"; \
    mkdir -p "/stage/$P" /out; \
    cp target/release/TeXora target/release/libpdfium.so README.md CREDITS.md LICENSE assets/TeXora.desktop "/stage/$P/"; \
    cp assets/icon.png "/stage/$P/TeXora.png"; \
    tar -C /stage -czf "/out/$P.tar.gz" "$P"

FROM debian:bookworm-slim AS validate
RUN apt-get update -qq && apt-get install -y -qq --no-install-recommends \
    libgl1 libegl1 libgl1-mesa-dri libglx-mesa0 libx11-6 libx11-xcb1 libxcb1 libxcb-randr0 \
    libxkbcommon0 libxkbcommon-x11-0 libxcursor1 libxi6 libxrandr2 xvfb xauth python3 >/dev/null
COPY --from=build /out /out
COPY scripts/in-linux-container.sh /validate.sh
RUN sh /validate.sh

FROM scratch AS export
COPY --from=validate /out /
