# Nautilus
<p>
<a href="https://www.syssec.ruhr-uni-bochum.de/media/emma/veroeffentlichungen/2018/12/17/NDSS19-Nautilus.pdf"> <img align="right" width="200"  src="https://github.com/RUB-SysSec/nautilus/raw/master/paper.png"> </a>

Nautilus is a feedback fuzzer inspired by AFL. However it allows to specify a grammar. Using this grammar, the fuzzer generates and internally uses the abstract syntax tree of the input. This also allows for very complex mutations. Then it converts the tree to the actual input.


<img width="400" align="center" src="https://github.com/RUB-SysSec/nautilus/raw/master/tree.png">

Knowing the exact tree shape greatly improves the performance for highly structured input formats, such as many text formats and programming languages. 

</p>
 

## Setup
```bash
# set workdir path
export WORKDIR="$(pwd)/nautilus"

# checkout the git:
git clone 'https://github.com/RUB-SysSec/nautilus.git'

# clang instrument wrapper
cd "$WORKDIR/forksrv/instrument/rt"
    sudo apt-get install g++-multilib # only if needed (error 'sys/cdefs.h' file not found)
    make #might need llvm-3.8-dev
cd "$WORKDIR/forksrv/instrument/clang_wrapper"
    make

# target
git clone https://github.com/mruby/mruby.git "$WORKDIR/forksrv/instrument/mruby"
cd "$WORKDIR/forksrv/instrument/mruby"
    sudo apt install ruby bison # if needed
    CC="$WORKDIR/forksrv/instrument/clang_wrapper/redqueen-clang" LD="$WORKDIR/forksrv/instrument/clang_wrapper/redqueen-clang" make
cd "$WORKDIR"

#update paths in config.ron
mkdir -p $WORKDIR/outputs/queue # if your workdir in the config is $WORKDIR, otherwise the fuzzer will crash because the queue is not found

#fix the paths in config.ron (line 14 to 16)

cargo run -p gramophone --release --bin fuzzer 
```
## Dockerfile

- Build the Dockerfile using the command `docker build . -t "nautilus:latest"`
- Run the dockerfile : `docker run -it nautilus:latest /bin/bash`
- Inside the docker image you can now run the same command `cargo run -p gramophone --release --bin fuzzer`
