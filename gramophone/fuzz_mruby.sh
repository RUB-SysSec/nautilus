#!/bin/bash

set -ue
cd $HOME/git/gramfuzz/gramophone

date
pushd .. >/dev/null
echo "Git commits:"
for r in *; do printf '%12s: ' $r; pushd $r >/dev/null; git rev-parse HEAD; popd >/dev/null; done
popd >/dev/null

git clean -xdf outputs/
cargo build --release
set -x
python local_snapshotter.py outputs $HOME/tmp/gfsnapshots \
  cargo run --release --bin fuzzer $HOME/git/gramfuzz/forksrv/instrument/mruby-asan/bin/mruby ../antlr_parser/src/ruby_grammar_less_recursion.json
