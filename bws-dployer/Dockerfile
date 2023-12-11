FROM rust:1-slim-bookworm

RUN apt-get update \
	&& apt-get install -y curl \
	&& rm -rf /var/lib/apt/lists/* \
	&& rm -rf /var/cache/apt/*

RUN cargo install cargo-quickinstall
RUN cargo quickinstall cargo-binstall
RUN cargo binstall bws -y
RUN cargo binstall nu -y

WORKDIR /deployer

COPY entrypoint.nu .

ENTRYPOINT ["./entrypoint.nu"]