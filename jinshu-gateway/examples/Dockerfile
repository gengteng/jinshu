FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/app-server .
COPY --from=builder jinshu/conf conf
EXPOSE 9200
ENTRYPOINT ["./app-server", "-r", "conf", "-c", "tracing", "database", "redis", "app", "gateway"]
