```bash
git clone 'https://redmine.trust.cased.de/git/gramfuzz_gramophone' gramophone
git clone 'https://redmine.trust.cased.de/git/gramfuzz_grammartec' grammartec
git clone 'https://redmine.trust.cased.de/git/gramfuzz_antlr_parser' antlr_parser
git clone 'https://redmine.trust.cased.de/git/gramfuzz_forksrv' forksrv
git clone 'https://redmine.trust.cased.de/git/gramfuzz_afl_mutator' afl_mutator
 cd forksrv/instrument/rt
  sudo apt-get install g++-multilib #only if needed (error 'sys/cdefs.h' file not found)
  make
 cd ../clang_wrapper
  # depending on your clang version you might have to replace 'const char * getPassName()' with
  # 'llvm::StringRef getPassName()' in afl-llvm-pass.cpp
  make
 cd ../
  git clone https://github.com/mruby/mruby.git
 cd mruby
  sudo apt install ruby bison #if needed
CC=$WORKDIR/gramfuzz_forksrv/instrument/clang_wrapper/redqueen-clang LD=$WORKDIR/gramfuzz_forksrv/instrument/clang_wrapper/redqueen-clang make
 cd ../../../gramophone 
 cargo run /path/to/binary [ grammar.json | grammar.g4 ]
```

# Commandline options

```
Usage: fuzzer [-g CONFIG] [-d] [grammar]

    -g CONFIG   Path to configuration file. Default: config.ron
    -d          Enable dumb mode
    grammar     Overwrite the grammar file specified in the CONFIG
```

## Run the fuzzer

```bash
cd $HOME/git/gramfuzz/gramophone
git clean -xdf outputs/
cargo build --release
python local_snapshotter.py outputs $HOME/tmp/gfsnapshots cargo run --release $HOME/git/gramfuzz/forksrv/instrument/mruby/bin/mruby ../antlr_parser/src/ruby_new_antlr_grammar.json
```



## Coverage

```bash
cd $HOME/git/gramfuzz/forksrv/instrument
git clone -b mruby-cov https://redmine.trust.cased.de/git/gramfuzz mruby-cov
cd mruby-cov
make CFLAGS='--coverage' LDFLAGS='--coverage'

cd $HOME/git/gramfuzz/gramophone/
git clone https://github.com/mrash/afl-cov.git

# wait a few hours/days

./snapshot_process_mruby.sh $HOME/tmp/gfsnapshots
```



## AFL

```bash
cd $HOME/git/gramfuzz/forksrv/instrument/mruby
git worktree add ../mruby-afl
cd ../mruby-afl
make CC=/usr/local/bin/afl-clang CFLAGS='-fPIC'

cd $HOME/git/gramfuzz/gramophone
python local_snapshotter.py ~/tmp/aflout ~/tmp/aflsnapshots afl/run_afl.sh


# wait a few hours/days

./snapshot_process_mruby.sh $HOME/tmp/aflsnapshots
```



## Coverage diff

```bash
python $HOME/git/gramfuzz/gramophone/lcov_diff.py $HOME/tmp/aflsnapshots/$timestamp/cov/lcov/trace.lcov_info_final $HOME/tmp/gfsnapshots/$timestamp/cov/lcov/trace.lcov_info_final tmp
genhtml --no-function-coverage --no-branch-coverage --output-directory $outputdir tmp
```



## ASan

```bash
cd $HOME/git/gramfuzz/forksrv/instrument/mruby
git worktree add ../mruby-asan
cd ../mruby-asan
make CC=$HOME/git/gramfuzz/forksrv/instrument/clang_wrapper/redqueen-clang LD=$HOME/git/gramfuzz/forksrv/instrument/clang_wrapper/redqueen-clang CFLAGS='-fsanitize=address' LDFLAGS='-fsanitize=address'

git worktree add ../mruby-asan-afl
cd ../mruby-asan-afl
make CC=/usr/local/bin/afl-clang CFLAGS='-fsanitize=address' LDFLAGS='-fsanitize=address'
make CC=/usr/local/bin/afl-clang CFLAGS='-fsanitize=address' LDFLAGS='-fsanitize=address' CFLAGS='-fPIC'
```



## Convert ANTLR to JSON
```bash
cd $HOME/git/gramfuzz/antlr_parser/src
cargo run ruby_antlr.g4 output.json
```
