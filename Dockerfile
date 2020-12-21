FROM rust:1.48 as builder

WORKDIR /usr/src/astg
COPY . .
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/astg/target/release/astg /usr/local/bin/astg
CMD ["astg"]