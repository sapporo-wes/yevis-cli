FROM rust:1.57.0-slim-buster

RUN apt update && \
    apt install -y --no-install-recommends \
    pkg-config && \
    libssl-dev && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

ENV RUST_BACKTRACE=1

ENTRYPOINT [""]
CMD ["sleep", "infinity"]