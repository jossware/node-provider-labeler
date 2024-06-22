FROM gcr.io/distroless/cc-debian12
ARG target=release
COPY ./target/${target}/node-provider-labeler /usr/local/bin/app
ENTRYPOINT ["/usr/local/bin/app"]
