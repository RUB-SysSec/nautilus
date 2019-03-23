#!/usr/bin/env bash

set -ue
cd "$(dirname "$0")"

#prog="$(readlink -e "../../forksrv/instrument/mruby-afl/bin/mruby")"
prog="$(readlink -e "../../forksrv/instrument/mruby-asan-afl/bin/mruby")"

in="$(readlink -e in)"
out="$HOME/tmp/aflout"
dict="$(readlink -e dict.txt)"

mkdir -p $HOME/tmp/afl{out,wd} && cd $HOME/tmp/aflwd

for f in fuzz_1 fuzz_2 fuzz_3 fuzz_4 fuzz_5 fuzz_6 fuzz_7; do
	afl-fuzz -i "$in" -o "$out" -x "$dict" -m none -S   $f   $prog @@ | gawk '{print "\033[33m['$f']\033[m", $0}' &
	sleep 1
done
	afl-fuzz -i "$in" -o "$out" -x "$dict" -m none -M fuzz_M $prog @@ | gawk '{print "\033[33m[fuzz_M]\033[m", $0}' &

wait
