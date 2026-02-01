#!/usr/bin/env bash

set -e

IMAGE_SIZE=$1
ROOT_DIR=$2
IMAGE=$3

echo
dd if=/dev/zero of=$IMAGE bs=$IMAGE_SIZE count=1
fakeroot mkfs.ext2 -i 16384 -d $ROOT_DIR $IMAGE

echo Created $IMAGE_SIZE EXT2 filesystem $IMAGE
