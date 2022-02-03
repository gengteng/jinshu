FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/jinshu-comet .
COPY --from=builder jinshu/conf conf
EXPOSE 9000
EXPOSE 9400
ENTRYPOINT ["./jinshu-comet", "-r", "conf", "-c", "tracing", "etcd", "redis", "comet"]
