FROM rust:1.57.0-slim-buster

RUN apt update && \
    apt install -y --no-install-recommends \
    pkg-config \
    libssl-dev && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

ADD https://download.docker.com/linux/static/stable/x86_64/docker-20.10.9.tgz /tmp/
RUN tar xf /tmp/docker-20.10.9.tgz -C /tmp && \
    mv /tmp/docker/* /usr/bin/ && \
    rmdir /tmp/docker && \
    rm -f /tmp/docker-20.10.9.tgz

WORKDIR /app
COPY . .

ENV RUST_BACKTRACE=1

ENTRYPOINT [""]
CMD ["sleep", "infinity"]