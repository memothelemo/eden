ARG USER=eden
ARG RUST_BUILD_MODE=release
ARG BUILD_DIR=/usr/build/eden

ARG COMMIT_HASH
ARG COMMIT_BRANCH

# We don't need to specify the Rust version since we're using nightly anyways
FROM lukemathwalker/cargo-chef:0.1.67-rust-bullseye AS chef
ARG BUILD_DIR
WORKDIR ${BUILD_DIR}

# Required dependencies for compilation
RUN apt-get update && \
    apt-get install -y \
        --no-install-recommends \
        cmake

#################################################################################
# So we don't have to download Rust nightly components all the time
FROM chef AS prepare

RUN cargo init
COPY rust-toolchain.toml .

RUN rm -rf src Cargo.toml .git .gitignore

FROM prepare AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

#################################################################################
FROM prepare AS compile

ARG BUILD_DIR
ARG RUST_BUILD_MODE

ARG COMMIT_HASH
ARG COMMIT_BRANCH

ENV VERGEN_GIT_SHA=${COMMIT_HASH} VERGEN_GIT_BRANCH=${COMMIT_BRANCH}

WORKDIR ${BUILD_DIR}

COPY --from=planner ${BUILD_DIR}/recipe.json recipe.json
RUN if [ "${RUST_BUILD_MODE}" = "debug" ]; then \
        cargo chef cook --recipe-path recipe.json; \
    elif [ "${RUST_BUILD_MODE}" = "release" ]; then \
        cargo chef cook --release --recipe-path recipe.json; \
    else \
        echo "Please specify whether RUST_BUILD_MODE is in 'debug' or 'release'"; \
        exit 1;\
    fi;

COPY . .
RUN if [ "${RUST_BUILD_MODE}" = "debug" ]; then \
        cargo build -p eden; \
    elif [ "${RUST_BUILD_MODE}" = "release" ]; then \
        cargo build --release -p eden; \
    else \
        echo "Please specify whether RUST_BUILD_MODE is in 'debug' or 'release'"; \
        exit 1;\
    fi;

#################################################################################
FROM debian:bullseye-slim AS runner

ARG RUST_BUILD_MODE
ARG BUILD_DIR
ARG USER

# Setup unprivileged user
RUN adduser \
    --disabled-password \
    --home "/dev/null" \
    --no-create-home \
    --gecos "" \
    ${USER}

WORKDIR /app

# Install required dependencies to run Eden (libpq is not required because we're using SQLx which it does not require libpq)
RUN apt update && apt install -y ca-certificates

COPY --from=compile --chmod=0755 ${BUILD_DIR}/target/${RUST_BUILD_MODE}/eden /app
USER ${USER}

ENTRYPOINT [ "./eden" ]
