FROM lukemathwalker/cargo-chef:latest-rust-1.58.1 as chef
WORKDIR /jinshu
RUN apt update && apt install cmake -y
RUN rustup component add rustfmt

FROM chef as planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef as builder

MAINTAINER "Geng Teng"

COPY --from=planner /jinshu/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
# Build our project
RUN cargo build --release --all-targets --examples

FROM debian:bullseye-slim AS target
WORKDIR /jinshu
COPY --from=builder /jinshu/target/release/jinshu-comet .
COPY --from=builder /jinshu/target/release/jinshu-authorizer .
COPY --from=builder /jinshu/target/release/jinshu-receiver .
COPY --from=builder /jinshu/target/release/jinshu-pusher .
COPY --from=builder /jinshu/target/release/jinshu-storage .
COPY --from=builder /jinshu/target/release/jinshu-timer .
COPY --from=builder /jinshu/target/release/jinshu-gateway .
COPY --from=builder /jinshu/target/release/jinshu-api .
COPY --from=builder /jinshu/target/release/jinshu-file .
COPY --from=builder /jinshu/target/release/jinshu-admin .
COPY --from=builder /jinshu/target/release/jinshu-cli .
COPY --from=builder /jinshu/target/release/examples/app-server .
COPY --from=builder /jinshu/conf conf