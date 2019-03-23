all: afl-llvm-pass.so symlinks

CXX=clang++
CC=clang
INCLUDES= -I$(shell readlink -f ../rt)
COMPILER_WRAPPER=$(shell readlink -f ./redqueen-clang) 

ifndef LLVM_CONFIG
	LLVM_CONFIG=llvm-config-3.8
endif

# Flags for building shared libraries.
SYSTEM=$(shell uname)
ifeq ($(SYSTEM), Darwin)
	CXXFLAGS= -fno-common 
	LDFLAGS= -dynamiclib -Wl,-flat_namespace -Wl,-undefined,suppress 
endif
ifeq ($(SYSTEM), Linux)
	CXXFLAGS= -fPIC #-D_GLIBCXX_USE_CXX11_ABI=0
	LDFLAGS= -shared 
endif

afl-llvm-pass.so : %.so: %.cpp
	$(CXX) $(CXXFLAGS) $< -o $@ `$(LLVM_CONFIG) --cxxflags` $(LDFLAGS) $(INCLUDES)

symlinks: 
	rm redqueen-clang++; true
	ln -s $(COMPILER_WRAPPER) redqueen-clang++

clean: 
	rm afl-llvm-pass.so;
	rm redqueen-clang++;
