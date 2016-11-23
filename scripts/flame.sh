#! /usr/bin/env bash
set -e
./target/release/universe &
PROGRAM=$!
sudo torch.sh -d 10 -o flamegraph.svg $PROGRAM && \
chromium-browser flamegraph.svg
