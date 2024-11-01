FROM rust:1.82

RUN apt-get update && apt-get install -y protobuf-compiler

WORKDIR /usr/src/myapp
COPY . .

RUN cargo install --path .

CMD ["template"]
