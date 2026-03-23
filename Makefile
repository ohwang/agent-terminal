.PHONY: build test install install-binary install-skill

build:
	cargo build --release

test:
	cargo test

install: install-binary install-skill

install-binary:
	cargo install --path .

install-skill:
	@./install-skill.sh
