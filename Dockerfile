FROM debian:stretch-slim

LABEL org.opencontainers.image.authors="DDBJ(DNA Data Bank of Japan) <t.ohta@nig.ac.jp>"
LABEL org.opencontainers.image.url="https://github.com/ddbj/yevis-cli"
LABEL org.opencontainers.image.source="https://github.com/ddbj/yevis-cli/blob/main/Dockerfile"
LABEL org.opencontainers.image.version="0.1.7"
LABEL org.opencontainers.image.description="CLI tool for registering workflows to ddbj/yevis-workflows"
LABEL org.opencontainers.image.licenses="Apache2.0"

ADD https://github.com/ddbj/yevis-cli/releases/latest/download/yevis /usr/bin/
RUN chmod +x /usr/bin/yevis

WORKDIR /app

ENTRYPOINT [""]
CMD ["sleep", "infinity"]