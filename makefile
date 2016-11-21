.PHONY:
all:
	cargo run
.PHONY:
fast:
	cargo run --release
.PHONY:
clip:
	cargo build --features dev
.PHONY:
perf:
	cargo build
	./scripts/flame.sh
