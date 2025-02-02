FROM rust:1.84-bookworm AS build
ENV PKG_CONFIG_ALLOW_CROSS=1

RUN USER=root cargo new --bin log
WORKDIR /log
RUN touch ./src/lib.rs
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release --bin log
RUN rm src/*.rs
RUN find target -name "*log*" -print0 | xargs -0 rm -rf

COPY ./migrations ./migrations
COPY ./src ./src
COPY .sqlx .sqlx

RUN cargo build --release --bin log

FROM debian:bookworm
WORKDIR /log
RUN apt-get update && apt-get install libsqlite3-dev -y
COPY --from=build /log/target/release/log log
CMD ["./log"]
