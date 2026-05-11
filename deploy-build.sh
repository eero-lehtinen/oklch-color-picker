#!/bin/bash

set -euxo pipefail

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && . "$HOME/.cargo/env"
rustup toolchain install stable --profile minimal
rustup target add wasm32-unknown-unknown
curl -sL https://github.com/thedodd/trunk/releases/latest/download/trunk-x86_64-unknown-linux-gnu.tar.gz | tar -xzf -
RUSTFLAGS='--cfg getrandom_backend="wasm_js"' ./trunk build --release
