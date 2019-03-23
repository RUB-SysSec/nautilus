FROM ubuntu

USER root
RUN apt-get update && apt-get install -y screen git build-essential clang cmake vim subversion curl 
RUN apt-get install -y screen git build-essential clang cmake screen automake libtool libtool-bin apt-utils autoconf zlib1g-dev pkg-config python-dev sl wget bison libglib2.0-dev ruby-dev
RUN apt-get install -y zlib1g-dev pkg-config python-dev sl
RUN apt-get install -y automake libtool autoconf gnupg
RUN apt-get install -y gawk g++ gcc make libreadline6-dev zlib1g-dev libssl-dev libyaml-dev libsqlite3-dev sqlite3 autoconf libgdbm-dev libncurses5-dev automake libtool bison pkg-config libffi-dev
RUN dpkg --add-architecture i386; apt-get update; apt-get install -y cmake g++ g++-multilib doxygen transfig imagemagick ghostscript git gdb flex texinfo libssl-dev:i386 
RUN apt-get install -y tmux psmisc htop binutils-dev libcurl4-openssl-dev zlib1g-dev libdw-dev libiberty-dev libgsl0-dev

RUN useradd -ms /bin/bash fuzzer
RUN chown -R fuzzer:fuzzer /home/fuzzer
USER fuzzer

#copy clangtool
COPY target /home/fuzzer/target
COPY instrument /home/fuzzer/instrument
USER root 
RUN chown -R fuzzer:fuzzer /home/fuzzer/*
USER fuzzer
WORKDIR /home/fuzzer
CMD /home/fuzzer/target/release/main /home/fuzzer/instrument/mruby/mruby/bin/mruby
