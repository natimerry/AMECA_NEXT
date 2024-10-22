FROM rust:slim-bullseye as build
LABEL authors="nat"
RUN USER=root apt-get update -y && apt-get -y install lld pkg-config libssl-dev
RUN USER=root cargo new --bin ameca_pg
RUN cargo install sqlx-cli
WORKDIR /ameca_pg

# 2. Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY . .
ENV SQLX_OFFLINE=true
RUN cargo sqlx prepare
RUN cargo build --release

RUN rm ./target/release/deps/ameca_pg*

FROM rust:slim-bullseye

WORKDIR /app
COPY --from=build /ameca_pg/target/release/ameca_pg ./ameca_pg
RUN touch ./.env
CMD ["./ameca_pg"]
