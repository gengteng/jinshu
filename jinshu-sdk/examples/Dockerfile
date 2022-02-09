FROM jinshu as builder
FROM debian:bullseye-slim AS runtime
MAINTAINER "Geng Teng"
WORKDIR jinshu
COPY --from=builder jinshu/app-client .
COPY --from=builder jinshu/conf conf
ENTRYPOINT ["./app-client", "-r", "conf", "-c", "tracing", "client", "app"]
