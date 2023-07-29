FROM rust:latest as builder

WORKDIR /app
ADD src /app/src
ADD Cargo.toml /app/Cargo.toml
ADD Cargo.lock /app/Cargo.lock
ADD Embed.toml /app/Embed.toml
ADD .cargo /app/.cargo
ADD .vscode /app/.vscode
ADD memory.x /app/memory.x
ADD openocd.gdp /app/openocd.gdp
ADD build.rs /app/build.rs
ADD build.sh /app/build.sh

RUN cargo build

FROM builder as runtime

CMD [ "./build.sh" ]
