ARG USER=eden
ARG RUST_BUILD_MODE=release
ARG BUILD_DIR=/usr/build/eden

# We don't need to specify the Rust version since we're using nightly anyways
FROM lukemathwalker/cargo-chef:0.1.67-rust-bullseye as chef
ARG BUILD_DIR
WORKDIR ${BUILD_DIR}

#################################################################################
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

#################################################################################
FROM chef AS compile
ARG BUILD_DIR
ARG RUST_BUILD_MODE
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
        cargo build; \
    elif [ "${RUST_BUILD_MODE}" = "release" ]; then \
        cargo build --release; \
    else \
        echo "Please specify whether RUST_BUILD_MODE is in 'debug' or 'release'"; \
        exit 1;\
    fi; \
    mkdir -p dist; \
    mv target/${RUST_BUILD_MODE}/eden dist/;

#################################################################################
FROM debian:bullseye-slim AS runner

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

COPY --chmod=0755 --from=compile ${BUILD_DIR}/dist/* ./
COPY --chmod=0755 --from=compile ${BUILD_DIR}/docker/entrypoint.sh ./

USER ${USER}

ENTRYPOINT [ "./entrypoint.sh" ]
