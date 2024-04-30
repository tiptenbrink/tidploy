#!/bin/bash
echoerr() { echo "$@" 1>&2; }
echo hello1
echo hello2
echoerr err1
echoerr err2
echo hello3