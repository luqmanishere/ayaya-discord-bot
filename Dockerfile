FROM rust:1.79 AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# install build dependencies
RUN apt update && apt install -y cmake 
RUN cargo install cargo-chef 
WORKDIR /usr/src/ayayadc

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
# Copy the build plan from the previous Docker stage
COPY --from=planner /usr/src/ayayadc/recipe.json recipe.json
# Build dependencies - this layer is cached as long as `recipe.json`
# doesn't change.
RUN cargo chef cook --release --recipe-path recipe.json
# Build the whole project
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runner
# install runtime dependencies
RUN apt-get update && apt-get install -y ffmpeg python3 pipx openssl libssl3 && rm -rf /var/lib/apt/lists/*
RUN pipx install yt-dlp
ENV PATH="${PATH}:/root/.local/bin"
# copy from builder
COPY --from=builder /usr/src/ayayadc/target/release/ayaya-discord-bot /usr/local/bin/ayaya-discord-bot
CMD ["ayaya-discord-bot"]

