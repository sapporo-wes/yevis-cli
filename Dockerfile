FROM debian:stretch-slim

LABEL org.opencontainers.image.authors="DDBJ(Bioinformatics and DDBJ Center) <t.ohta@nig.ac.jp>"
LABEL org.opencontainers.image.url="https://github.com/ddbj/yevis-cli"
LABEL org.opencontainers.image.source="https://github.com/ddbj/yevis-cli/blob/main/Dockerfile"
LABEL org.opencontainers.image.version="0.3.0"
LABEL org.opencontainers.image.description="CLI tool to support building and maintaining Yevis workflow registry"
LABEL org.opencontainers.image.licenses="Apache2.0"

RUN apt update && \
    apt install -y --no-install-recommends \
    ca-certificates \
    curl && \
    apt clean && \
    rm -rf /var/lib/apt/lists/*

RUN curl -fsSL -o /tmp/docker.tgz https://download.docker.com/linux/static/stable/$(uname -m)/docker-20.10.9.tgz && \
    tar -C /tmp -xf /tmp/docker.tgz && \
    mv /tmp/docker/* /usr/bin/ && \
    rm -rf /tmp/docker /tmp/docker.tgz

ADD https://github.com/ddbj/yevis-cli/releases/latest/download/yevis /usr/bin/
RUN chmod +x /usr/bin/yevis

WORKDIR /app

ENTRYPOINT ["yevis"]
CMD [""]