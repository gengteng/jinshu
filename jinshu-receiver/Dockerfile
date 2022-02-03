FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/jinshu-receiver .
COPY --from=builder jinshu/conf conf
EXPOSE 9100
ENTRYPOINT ["./jinshu-receiver", "-r", "conf", "-c", "tracing", "etcd", "kafka", "receiver"]
