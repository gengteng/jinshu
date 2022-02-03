FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/jinshu-pusher .
COPY --from=builder jinshu/conf conf
EXPOSE 9100
ENTRYPOINT ["./jinshu-pusher", "-r", "conf", "-c", "tracing", "etcd", "redis", "pusher", "kafka"]
