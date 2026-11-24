# syntax=docker/dockerfile:1.7
# What rstyle-mcp-server's Check image takes from this repository: cargo-rstyle, rstyle-driver and rstyle, built for
# release on rstyle's base image (rstyled/rstyle-base), which the Check image runs them on. rstyle-driver links rustc's
# internals, so it runs only with the nightly it was built with: the base's, which rust-toolchain.toml must name. The
# image holds only the three binaries, at the root. .github/workflows/publish.yml publishes it as
# ghcr.io/rstyled/rstyle:<commit>.
FROM ghcr.io/rstyled/rstyle-base:9c106b6fac70ff5e09f28623da3e7ed0c1ba3cc6 AS build
COPY . /src
WORKDIR /src
RUN pinned="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)" && [ "$pinned" = "$RUSTUP_TOOLCHAIN" ] || { echo "rust-toolchain.toml names $pinned, but the base has $RUSTUP_TOOLCHAIN: bump rstyled/rstyle-base first" >&2; exit 1; }
# rstyle-driver's build.rs bakes in this commit, read from .git, which the context keeps (Dockerfile.dockerignore).
RUN cargo build --release --locked --bin cargo-rstyle --bin rstyle-driver --bin rstyle && target/release/cargo-rstyle --help >/dev/null && target/release/rstyle --help >/dev/null

FROM scratch
LABEL org.opencontainers.image.source=https://github.com/rstyled/rstyle
COPY --from=build /src/target/release/cargo-rstyle /src/target/release/rstyle-driver /src/target/release/rstyle /
