#!/bin/bash

. "$HOME/.cargo/env"

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
cargo install --locked trunk
rustup target add wasm32-unknown-unknown
cargo install --force cargo-make