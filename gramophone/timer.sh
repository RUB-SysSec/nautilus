#!/usr/bin/env bash

reset() {
	echo -en '\033k\033\\' 
	exit 1
}

trap reset INT

while true; do
	printf '\033k%d:%02d\033\\' $(($SECONDS / 60)) $(($SECONDS % 60))
	sleep 1
done
