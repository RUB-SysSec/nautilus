#!/bin/bash

set -uxe

kill -- -$(cat ~/tmp/gfpid)
sleep 1
kill -9 -- -$(cat ~/tmp/gfpid)
