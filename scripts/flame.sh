#! /usr/bin/env bash
set -e
./target/debug/universe &
PROGRAM=$!
sleep 4s
sudo torch.sh -d 20 -o flamegraph.svg $PROGRAM && \
chromium-browser flamegraph.svg
