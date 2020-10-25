#!/bin/sh

# TODO: makefile

cargo build -j 8 --release  --features vendored  --target x86_64-unknown-linux-musl
