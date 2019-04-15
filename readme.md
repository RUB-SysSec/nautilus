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
mkdir $WORKDIR/outputs/queue # if your workdir in the config is $WORKDIR, otherwise the fuzzer will crash because the queue is not found
		cargo run -p gramophone --release --bin fuzzer 
```

## Project Structure

### Git
All dependencies are added as `git submodule`.  
These are just git repos referenced in the parent repo.  
Commits made in submodules must be pushed as they are referenced in the parent repository.

The following commands will automatically include submodules in pull / push operations: 

#### Pull
```bash
git pull --recurse-submodules
```

#### Push
```bash
git push --recurse-submodules=on-demand
```


### Cargo
The cargo file in the parent repository contains a workspace with all sub-projects.  
You can simply add `-p $PROJECT` to run `cargo` within the selected project.

#### Example
```bash
cargo build --release # will build all projects
cargo -p gramophone build --release # will build only gramophone (and dependencies)
```


## Run the fuzzer

```bash
cd "$WORKDIR"
git clean -xdf outputs/
cargo build --release
python scripts/local_snapshotter.py outputs $HOME/tmp/gfsnapshots cargo -p gramophone run --release --bin fuzzer -- forksrv/instrument/mruby/bin/mruby antlr_parser/src/ruby_new_antlr_grammar.json
```



## Coverage

```bash
cd "$WORKDIR/forksrv/instrument"
git clone -b mruby-cov https://redmine.trust.cased.de/git/gramfuzz mruby-cov
cd mruby-cov
make CFLAGS='--coverage' LDFLAGS='--coverage'

cd "$WORKDIR"
git clone https://github.com/mrash/afl-cov.git

# wait a few hours/days

./snapshot_process_mruby.sh $HOME/tmp/gfsnapshots
```



## AFL

```bash
cd "$WORKDIR/forksrv/instrument/mruby"
git worktree add ../mruby-afl
cd ../mruby-afl
make CC=/usr/local/bin/afl-clang CFLAGS='-fPIC'

cd "$WORKDIR"
python scripts/local_snapshotter.py ~/tmp/aflout ~/tmp/aflsnapshots afl/run_afl.sh


# wait a few hours/days

./snapshot_process_mruby.sh $HOME/tmp/aflsnapshots
```



## Coverage diff

```bash
python "$WORKDIR/scripts/lcov_diff.py" $HOME/tmp/aflsnapshots/$timestamp/cov/lcov/trace.lcov_info_final $HOME/tmp/gfsnapshots/$timestamp/cov/lcov/trace.lcov_info_final tmp
genhtml --no-function-coverage --no-branch-coverage --output-directory $outputdir tmp
```



## ASan

```bash
cd "$WORKDIR/forksrv/instrument/mruby"
git worktree add ../mruby-asan
cd ../mruby-asan
make CC="$WORKDIR/forksrv/instrument/clang_wrapper/redqueen-clang" LD="$WORKDIR/forksrv/instrument/clang_wrapper/redqueen-clang" CFLAGS='-fsanitize=address' LDFLAGS='-fsanitize=address'

git worktree add ../mruby-asan-afl
cd ../mruby-asan-afl
make CC=/usr/local/bin/afl-clang CFLAGS='-fsanitize=address' LDFLAGS='-fsanitize=address'
make CC=/usr/local/bin/afl-clang CFLAGS='-fsanitize=address' LDFLAGS='-fsanitize=address' CFLAGS='-fPIC'
```



## Convert ANTLR to JSON
```bash
cd "$WORKDIR"
cargo run -p antlr_parser antlr_parser/src/ruby_antlr.g4 output.json
```
