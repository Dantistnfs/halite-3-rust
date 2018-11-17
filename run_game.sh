#!/usr/bin/env bash

set -e

cargo build --release
./halite -i replays/ -vvv  --width 64 --height 64 "RUST_BACKTRACE=1 ./target/release/my_bot" "RUST_BACKTRACE=1 ./previous_version/my_bot" #"RUST_BACKTRACE=1 ./previous_version/my_bot" "RUST_BACKTRACE=1 ./previous_version/my_bot"
