FROM rust:slim-bullseye as build
LABEL authors="nat"
RUN USER=root apt-get update -y && apt-get -y install pkg-config libssl-dev
RUN USER=root cargo new --bin mempaste-api
WORKDIR /ameca

# 2. Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
COPY ./migrations ./migrations
RUN cargo build --release

RUN rm ./target/release/deps/mempaste_api*

FROM rust:slim-bullseye

WORKDIR /bot
COPY --from=build /ameca/target/release/ameca_pg ./ameca_pg
RUN touch ./.env
CMD ["./ameca_pg"]
