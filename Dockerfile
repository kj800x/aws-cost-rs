FROM rust:1.74

WORKDIR /usr/src/aws-cost-rs
COPY . .

RUN cargo install --path .

CMD ["aws-cost-rs"]
