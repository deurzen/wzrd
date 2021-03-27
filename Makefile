all: build

build: tags
	cargo build

test:
	cargo test

debug: build
	./launch

release:
	RUSTFLAGS="--emit=asm" cargo build --release

install:
	install ./target/release/wzrd /usr/local/bin/wzrd

.PHONY: tags
tags:
	ctags -R --exclude=.git --exclude=target --fields=+iaS --extras=+q .

.PHONY: format
format:
	@cargo +nightly fmt
