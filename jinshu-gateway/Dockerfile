FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/jinshu-gateway .
COPY --from=builder jinshu/conf conf
EXPOSE 9200
ENTRYPOINT ["./jinshu-gateway", "-r", "conf", "-c", "tracing", "database", "redis", "gateway"]
