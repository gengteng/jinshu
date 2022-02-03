FROM jinshu as jinshu
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=jinshu jinshu/jinshu-authorizer .
COPY --from=jinshu jinshu/conf conf
EXPOSE 9300
ENTRYPOINT ["./jinshu-authorizer", "-r", "conf", "-c", "tracing", "etcd", "redis", "authorizer"]
