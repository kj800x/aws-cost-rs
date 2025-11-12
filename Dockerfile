FROM rust:1.91

WORKDIR /usr/src/aws-cost-rs
COPY . .

RUN cargo install --path .

CMD ["aws-cost-rs"]
