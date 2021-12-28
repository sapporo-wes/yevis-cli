FROM rust:1.57.0-slim-buster

WORKDIR /app
COPY . .

ENV RUST_BACKTRACE=1

ENTRYPOINT [""]
CMD ["sleep", "infinity"]