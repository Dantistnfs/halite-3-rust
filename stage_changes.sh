#!/usr/bin/env bash

./run_game.sh # builds and runs one game
rm -f ./previous_version/my_bot
cp ./target/debug/my_bot ./previous_version/my_bot

zip -r my_bot.zip ./src Cargo.toml
