FROM ubuntu:16.04
FROM liuchong/rustup:nightly
RUN apt-get -y update --fix-missing
ENV DEBIAN_FRONTEND=noninteractive

WORKDIR /root

RUN apt-get install -y vim git-all build-essential gcc-multilib g++-multilib \
ruby bison curl 
RUN git clone https://github.com/RUB-SysSec/nautilus.git
RUN apt-get install -y clang-3.8
RUN ln -s /usr/bin/clang-3.8 /usr/bin/clang
RUN ln -s /usr/bin/clang++-3.8 /usr/bin/clang++

WORKDIR "/root/nautilus/forksrv/instrument/rt"
RUN make
WORKDIR "/root/nautilus/forksrv/instrument/clang_wrapper"
RUN make

WORKDIR "/root/nautilus/forksrv/instrument/"
RUN git clone https://github.com/mruby/mruby.git "mruby"
WORKDIR "/root/nautilus/forksrv/instrument/mruby"
RUN CC="/root/nautilus/forksrv/instrument/clang_wrapper/redqueen-clang" LD="/root/nautilus/forksrv/instrument/clang_wrapper/redqueen-clang" make

WORKDIR "/root/nautilus"
RUN mkdir -p outputs/queue
COPY config-ron.patch ./
RUN git apply config-ron.patch
RUN cargo build -p gramophone --release --bin fuzzer
