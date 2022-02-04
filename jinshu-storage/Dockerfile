FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/jinshu-storage .
COPY --from=builder jinshu/conf conf
ENTRYPOINT ["./jinshu-storage", "-r", "conf", "-c", "tracing", "kafka", "database"]
