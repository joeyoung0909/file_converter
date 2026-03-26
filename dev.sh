#!/bin/bash
# Development startup script
# Uses the Rust 1.94 installation at ~/.cargo_new

export RUSTUP_HOME="$HOME/.rustup_new"
export CARGO_HOME="$HOME/.cargo_new"
export PATH="$HOME/.cargo_new/bin:$PATH"

echo "Using Rust: $(cargo --version)"
echo "Using Cargo: $(cargo --version)"

npm run tauri dev
