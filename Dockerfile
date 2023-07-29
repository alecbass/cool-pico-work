FROM rust:latest as setup

WORKDIR /app
ADD src /app/src
ADD Cargo.toml /app/Cargo.toml
ADD Cargo.lock /app/Cargo.lock
ADD Embed.toml /app/Embed.toml
ADD .cargo /app/.cargo
ADD .vscode /app/.vscode
ADD memory.x /app/memory.x
ADD openocd.gdb /app/openocd.gdb
ADD build.rs /app/build.rs
ADD build.sh /app/build.sh
ADD openocd-config /app/openocd-config

FROM setup as hardware

# Add hardware requirements

RUN rustup target add thumbv6m-none-eabi
RUN cargo install flip-link
RUN cargo build

ADD ./install-deps.sh /app/install-deps.sh
RUN ./install-deps.sh

FROM hardware as runtime

# CMD [ "./build.sh" ]
