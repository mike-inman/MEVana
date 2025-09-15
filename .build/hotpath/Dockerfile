FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates librdkafka1 curl \
    && update-ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY hotpath_arb_engine /usr/local/bin/hotpath_arb_engine
ENV RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/hotpath_arb_engine"]
