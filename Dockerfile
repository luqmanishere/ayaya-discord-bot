FROM rust:latest AS chef 
# We only pay the installation cost once, 
# it will be cached from the second build onwards
# install build dependencies
RUN apt update && apt install -y cmake nodejs npm
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
WORKDIR /usr/src/ayayadc/dashboard
RUN npm install
RUN npm run build
WORKDIR /usr/src/ayayadc
RUN cargo build --release

FROM debian:trixie-slim AS runner
# install runtime dependencies
RUN apt-get update && apt-get install -y ffmpeg python3 pipx openssl libssl3 nodejs && rm -rf /var/lib/apt/lists/*
RUN pipx install --pip-args "\\--pre" "yt-dlp[default]"
ENV PATH="${PATH}:/root/.local/bin"
# copy from builder
COPY --from=builder /usr/src/ayayadc/target/release/ayaya-runner-local /usr/local/bin/ayaya-runner-local
CMD ["ayaya-runner-local"]


