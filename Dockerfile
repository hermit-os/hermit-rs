FROM debian:buster

RUN apt-get update && \
    apt-get -y install cpu-checker
