#!/usr/bin/env bash

set -ue #x
cd "$(dirname "$0")"

snapsdir="$1"

total=$(echo "$snapsdir"/?????/ | wc -w)
processed=0

for snapdir in "$snapsdir"/?????/; do
	for (( i = 0; i < processed; i++ )); do echo -n █; done
	: $((i++)); echo -n ▒
	for (( ; i < total; i++ )); do echo -n ░; done
	echo " $snapdir"

	find "$snapdir" -name \*er:Timeouted -exec chmod 0 {} +

	afl-cov/afl-cov -d "$snapdir" -v --coverage-at-exit --enable-branch-coverage -O \
		--coverage-cmd "$PWD/../forksrv/instrument/mruby-cov/bin/mruby 'AFL_FILE'" \
		--code-dir "$PWD/../forksrv/instrument/mruby-cov/build/host/src/" \
		2>&1 >"$snapdir"/coverage.log

	tail -n 5 "$snapdir"/coverage.log | head -n 3
	printf "%s\t%s\t%s\t%s\t%s\t%s\t%s\n" "$(basename "$snapdir")" \
		$(tail -n 5 "$snapdir"/coverage.log | grep 'lines'     | awk '{print substr($3,2), $5}' ) \
		$(tail -n 5 "$snapdir"/coverage.log | grep 'functions' | awk '{print substr($3,2), $5}' ) \
		$(tail -n 5 "$snapdir"/coverage.log | grep 'branches'  | awk '{print substr($3,2), $5}' ) \
		>> "$snapsdir"/coverage.tsv

	: $(( processed++ ))
done


