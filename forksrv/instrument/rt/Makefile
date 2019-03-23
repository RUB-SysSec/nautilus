all: runtime

runtime:
	clang -Ofast -fPIC -ggdb3 -c -o common.o common.c
	clang -Ofast -m32 -ggdb3 -c -o common_32.o common.c
	clang -Ofast -m64 -ggdb3 -c -o common_64.o common.c
clean:
	rm -f *.so test
	rm -f *.a test
