#!/bin/bash
echoerr() { echo "$@" 1>&2; }
echo hello1
echo hello2
sleep 2
echo hello3
sleep 1
echoerr hello world2
echoerr hello world3
sleep 1
echoerr hello world1