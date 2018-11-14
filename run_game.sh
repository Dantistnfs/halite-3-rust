#!/usr/bin/env bash

set -e

cargo build
./halite --replay-directory replays/ -vvv --width 64 --height 64 "RUST_BACKTRACE=1 ./target/debug/my_bot" "RUST_BACKTRACE=1 ./previous_version/my_bot"
