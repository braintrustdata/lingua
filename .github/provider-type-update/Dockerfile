FROM node:24-bookworm-slim@sha256:55fb5f5a527ea2c5c4a95f963e48ef90aa38c7b346054fdc8ee9957a2b55d205

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential ca-certificates clang cmake curl git libssl-dev pkg-config \
    protobuf-compiler python3 ripgrep unzip \
    && rm -rf /var/lib/apt/lists/*
ENV RUSTUP_HOME=/opt/rustup CARGO_HOME=/opt/cargo
ENV PATH=/opt/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/rustup.sh \
    && sh /tmp/rustup.sh -y --profile minimal --component rustfmt,clippy \
    && rustup target add wasm32-unknown-unknown \
    && cargo install wasm-pack --version 0.14.0 --locked \
    && rm /tmp/rustup.sh
RUN npm install -g pnpm@10.33.0 @anthropic-ai/claude-code@2.1.304

RUN mkdir /workspace /artifacts \
    && chown node:node /workspace /artifacts
COPY scripts/ /opt/updater/
COPY prompts/ /opt/updater/prompts/
ENV HOME=/home/node CARGO_HOME=/home/node/.cargo RUNNER_TEMP=/tmp/provider-type-update
WORKDIR /workspace
USER 1000:1000
CMD ["sleep", "infinity"]
