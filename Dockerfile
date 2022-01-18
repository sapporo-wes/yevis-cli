FROM debian:stretch-slim

ADD https://github.com/ddbj/yevis-cli/releases/latest/download/yevis /usr/bin/
RUN chmod +x /usr/bin/yevis

WORKDIR /app

ENTRYPOINT [""]
CMD ["sleep", "infinity"]