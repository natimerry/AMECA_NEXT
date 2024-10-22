FROM rust:slim-bullseye as build
LABEL authors="nat"
RUN USER=root apt-get update -y && apt-get -y install pkg-config libssl-dev lld
RUN USER=root cargo new --bin ameca_pg
WORKDIR /ameca_pg

# 2. Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY . .
RUN cargo build --release

FROM rust:slim-bullseye
WORKDIR /app
RUN touch ./.env
COPY --from=build /ameca_pg/target/release/ameca_pg ./ameca_pg
CMD ["./ameca_pg"]