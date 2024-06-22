FROM gcr.io/distroless/cc-debian12
COPY ./target/release/node-provider-labeler /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]
