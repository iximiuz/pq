FROM rust:1.63.0 AS builder


RUN cargo install --git https://github.com/iximiuz/pq

FROM debian:11.4-slim

# Make this image whalebrew compatible for easier usage: https://github.com/whalebrew/whalebrew.
LABEL io.whalebrew.name 'pq'
LABEL io.whalebrew.config.working_dir '$PWD'

COPY --from=builder /usr/local/cargo/bin/pq /usr/local/bin/pq
ENTRYPOINT ["pq"]
