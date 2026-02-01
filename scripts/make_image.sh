#!/usr/bin/env bash

set -e

IMAGE=$1
shift

PART_COUNT=0
# There are 34 blocks of overhead at the start and 33 at the end.
IMAGE_BLOCKSZ=67

announce_actions() {
    local IDX=1
    while [ "$1" != "" ]; do
        DESC=$1; NAME=$2; BLOB=$3; TYPE=$4; shift 4
        local BYTESZ=`stat --printf='%s' $BLOB`
        local BLOCKSZ=$[(BYTESZ + 511) / 512]
        
        echo "  $IDX: $DESC ($BLOCKSZ blocks)"
        
        IDX=$[IDX + 1]
    done
}

count_size() {
    while [ "$1" != "" ]; do
        DESC=$1; NAME=$2; BLOB=$3; TYPE=$4; shift 4
        local BYTESZ=`stat --printf='%s' $BLOB`
        local BLOCKSZ=$[(BYTESZ + 511) / 512]
        
        IMAGE_BLOCKSZ=$[IMAGE_BLOCKSZ + BLOCKSZ]
        PART_COUNT=$[PART_COUNT + 1]
    done
}

format_gpt() {
    local IDX=1
    local OFFSET=34
    local ARGS=''
    while [ "$1" != "" ]; do
        DESC=$1; NAME=$2; BLOB=$3; TYPE=$4; shift 4
        local BYTESZ=`stat --printf='%s' $BLOB`
        local BLOCKSZ=$[(BYTESZ + 511) / 512]
        
        local PARTEND=$[OFFSET + BLOCKSZ - 1]
        ARGS="$ARGS --new=$IDX:$OFFSET:$PARTEND --change-name=$IDX:$NAME --typecode=$IDX:$TYPE"
        
        OFFSET=$[OFFSET + BLOCKSZ]
        IDX=$[IDX + 1]
    done
    echo "$ARGS"
    sgdisk -a 1 $ARGS $IMAGE
}

copy_content() {
    local OFFSET=34
    local ARGS=''
    while [ "$1" != "" ]; do
        DESC=$1; NAME=$2; BLOB=$3; TYPE=$4; shift 4
        local BYTESZ=`stat --printf='%s' $BLOB`
        local BLOCKSZ=$[(BYTESZ + 511) / 512]
        
        echo "$BLOB -> $IMAGE @ $OFFSET"
        dd if=$BLOB of=$IMAGE bs=512 seek=$OFFSET conv=notrunc
        
        OFFSET=$[OFFSET + BLOCKSZ]
    done
}

count_size "$@"

echo
echo
echo "Formatting $IMAGE_BLOCKSZ block image with $PART_COUNT partitions:"
announce_actions "$@"
echo

dd if=/dev/zero of=$IMAGE bs=512 count=$IMAGE_BLOCKSZ
format_gpt "$@"
copy_content "$@"

echo
echo Image $IMAGE created successfully
echo
