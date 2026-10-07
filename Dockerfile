# ---- build stage ----
FROM rust:1-slim-bookworm AS build
WORKDIR /app

# Build dependencies first so they're cached between code changes
COPY Cargo.toml ./
RUN mkdir src static && echo "fn main() {}" > src/main.rs && touch static/index.html \
    && cargo build --release && rm -rf src static

COPY src ./src
COPY static ./static
RUN touch src/main.rs && cargo build --release

# ---- runtime stage ----
FROM debian:bookworm-slim
COPY --from=build /app/target/release/fractal-explorer /usr/local/bin/fractal-explorer
ENV PORT=8080
EXPOSE 8080
USER nobody
CMD ["fractal-explorer"]
