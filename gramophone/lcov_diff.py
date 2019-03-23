# coding: utf-8

import sys


def read_entries(firstf):
	lines_hit = set()

	with open(firstf, 'r') as f:
		sf = None

		for line in f:
			if line == 'end_of_record':
				sf = None

			elif line.startswith('SF:'):
				sf = line[3:-1]

			elif line.startswith('DA:'):
				ln, hits = line[3:-1].split(',')
				if hits != '0':
					# print sf, '§', ln, repr(hits)
					lines_hit.add((sf, ln))

	# print lines_hit

	return lines_hit


def remove_lines(lines_hit, secondf, outf):
	with open(secondf, 'r') as f, open(outf, 'w') as out:
		sf = None

		for line in f:
			if line == 'end_of_record':
				sf = None

			elif line.startswith('SF:'):
				sf = line[3:-1]

			if line.startswith('DA:'):
				ln, hits = line[3:-1].split(',')
				if (sf, ln) in lines_hit:
					out.write('DA:{},0\n'.format(ln))
					continue

			out.write(line)




def main():
	lines_hit = read_entries(sys.argv[1])
	# print lines_hit
	remove_lines(lines_hit, sys.argv[2], sys.argv[3])


if __name__ == '__main__':
	main()
