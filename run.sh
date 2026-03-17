#!/usr/bin/env sh
set -eu

cargo build --release
./target/release/hive_node
