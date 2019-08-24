

FROM rustlang/rust:nightly as builder
WORKDIR /root
COPY . /root
RUN cargo build --release

FROM ubuntu
EXPOSE 53/udp
WORKDIR /root
COPY --from=builder ./root/target/release/updns .
CMD ["./updns"]


