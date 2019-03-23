#!/bin/bash

cd $HOME/git/gramfuzz/gramophone
rm -rfv ~/tmp/gf*
git clean -xdf outputs
echo $BASHPID > $HOME/tmp/gfpid

nohup $HOME/git/gramfuzz/gramophone/fuzz_mruby.sh 2>&1 > $HOME/tmp/gflog &
