FROM rust:buster as builder
WORKDIR /usr/src/muminst-rust-server
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y libopus-dev ffmpeg sqlite3 youtube-dl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/muminst-rust-server /usr/local/bin/muminst-rust-server
USER nobody 
EXPOSE 8080
ENTRYPOINT ["muminst-rust-server"]
