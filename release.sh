#!/bin/bash
set -e
set -x
cargo build --release

PLATFORM=`uname`
if [ "$PLATFORM" = "Darwin" ]; then
  SUFFIX="macOS"
elif [ "$PLATFORM" = "Linux" ]; then
  SUFFIX="Linux"
else
    echo "Unknown platform"
    exit 1
fi

tar cz target/release/ilvm > ilvm-$SUFFIX.tar.gz