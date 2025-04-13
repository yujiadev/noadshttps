FROM rust:latest

WORKDIR /usr/src/app

COPY . .

RUN cargo build --release

RUN target/release/noadshttps init configs.toml

EXPOSE 5000

CMD ["target/release/noadshttps", "run", "configs.toml"]