FROM rust:latest as setup

ARG USERNAME=pico
ARG GROUPID=1000
ARG USERID=1000
ENV HOME=/app

WORKDIR /app

RUN useradd --uid ${USERID} ${USERNAME}

FROM setup as hardware
# Add hardware requirements

RUN rustup target add thumbv6m-none-eabi
RUN cargo install flip-link

RUN mkdir /home/pico

ADD ./setup-pico.sh /app/setup-pico.sh
RUN ./setup-pico.sh

ADD ./install-deps.sh /app/install-deps.sh
RUN ./install-deps.sh

FROM hardware as build

# Add project
ADD src /app/src
ADD Cargo.toml /app/Cargo.toml
ADD Cargo.lock /app/Cargo.lock
ADD Embed.toml /app/Embed.toml
ADD Makefile /app/Makefile
ADD .cargo /app/.cargo
ADD memory.x /app/memory.x
ADD openocd.gdb /app/openocd.gdb
ADD build.rs /app/build.rs
ADD CMakeLists.txt /app/CMakeLists.txt
ADD run-minicom.sh /app/run-minicom.sh
ADD run-openocd.sh /app/run-openocd.sh

# ADD build.sh /app/build.sh
ADD udev /app/udev


FROM build as runtime

# Setup USB rules
RUN udevadm control --reload-rules || echo "done"
# RUN udevadm trigger

RUN cargo build
RUN chown -R ${USERNAME}:${USERNAME} /app
USER ${USERNAME}

EXPOSE 3333

# RUN source .env

# CMD [ "./build.sh" ]

# To launch with udev and USB volumes, run
# docker run -it -v=./udev/rules.d:/etc/udev/rules.d -v=/dev/bus/usb:/dev/bus/usb pico
