#!/usr/bin/env bash

set -e

cargo build --release
./halite -i replays/ -vvv  --width 64 --height 64 "RUST_BACKTRACE=1 ./target/release/my_bot 6 120 30 100 15 8 20 175" "RUST_BACKTRACE=1 ./previous_version/my_bot" #"RUST_BACKTRACE=1 ./previous_version/my_bot" "RUST_BACKTRACE=1 ./previous_version/my_bot"
