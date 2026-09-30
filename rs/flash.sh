#!/bin/sh

disk=/dev/disk/by-label/RPI-RP2
dev=/dev/ttyACM0

mkdir -p .rpi

[ ! -e "$disk" ] && echo "Board not connected or not in bootloader mode"
while [ ! -e "$disk" ]; do sleep 0.2; done

[ "$(/bin/ls -A .rpi)" ] && {
        echo "A board is already mounted"
        exit 1
}

sudo mount -o uid=$(id -u),sync $disk .rpi

elf2uf2-rs -d "$1" || exit 1
