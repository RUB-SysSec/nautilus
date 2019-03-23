from subprocess import Popen, call, check_call
from time import time, sleep
from datetime import datetime
import os
import sys
import math


def snapshot_times():
	n = 1
	while True:
		yield n
		if n <= 120:
			n = int(math.ceil(n * 1.2))
		else:
			n += 60
			n -= n % 60


def main():
	directory_to_check = sys.argv[1]
	snapshot_directory = sys.argv[2]
	command = sys.argv[3:]

	check_call(["mkdir", "-p", snapshot_directory])

	start = time()
	Popen(command)
	Popen("./timer.sh")

	for t in snapshot_times():
		sleep(start + t * 60 - time())
		print "\nTaking snapshot, t={}m, {} ".format(t, datetime.now())
		dirname = os.path.join(snapshot_directory, "{:05}".format(t))
		while not 0 == call(["cp", "-rl", directory_to_check, dirname]):
			call(["rm", "-r", dirname])


if __name__ == '__main__':
	main()
