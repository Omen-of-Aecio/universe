.PHONY:
all:
	cargo run
.PHONY:
fast:
	cargo run --release
.PHONY:
clip:
	cargo build --features dev
