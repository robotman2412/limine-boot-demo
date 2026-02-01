#!/usr/bin/env bash

set -e

IMAGE_SIZE=$1
ROOT_DIR=$2
IMAGE=$3

echo
dd if=/dev/zero of=$IMAGE bs=$IMAGE_SIZE count=1 >/dev/null
mformat -i $IMAGE
mcopy -s -i $IMAGE `find $ROOT_DIR/ -mindepth 1 -maxdepth 1` '::/'
echo

echo Created $IMAGE_SIZE FAT filesystem $IMAGE
