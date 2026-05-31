FROM rust:1.96

WORKDIR /usr/src/aws-cost-rs
COPY . .

RUN cargo install --path .

CMD ["aws-cost-rs"]
