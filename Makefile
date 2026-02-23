# Copyright © 2026, __robot@PLT
# SPDX-License-Identifier: MIT

EFI_PART_SIZE ?= 4MiB
CROSS_COMPILE ?= 
QEMU ?= qemu-system-x86_64
QEMU_FLAGS ?=

.PHONY: image
image: sysroot
	mkdir -p build/image
	./scripts/make_fatfs.sh $(EFI_PART_SIZE) build/efiroot build/image/efi.fatfs
	./scripts/make_image.sh \
		build/image.hdd \
		'EFI partition'  boot build/image/efi.fatfs 0x0700

.PHONY: qemu
qemu: edk2-ovmf image
	$(QEMU) $(QEMU_FLAGS) -s \
		-drive if=pflash,unit=0,format=raw,file=edk2-ovmf/ovmf-code-x86_64.fd \
		-drive format=raw,file=build/image.hdd,cache=none \
	| scripts/address-filter.py -L -A $(CROSS_COMPILE)addr2line target/x86_64-unknown-none/debug/limine-boot-demo

edk2-ovmf:
	curl -L https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/edk2-ovmf.tar.gz | gunzip | tar -xf -

.PHONY: gdb
gdb:
	$(CROSS_COMPILE)gdb -x misc/gdbinit target/x86_64-unknown-none/debug/limine-boot-demo

.PHONY: sysroot
sysroot: kernel
	# EFI root folders
	mkdir -p build/efiroot/EFI/BOOT
	mkdir -p build/efiroot/boot
	
	# Copy the bootloader, modules and kernel into the EFI root
	cp limine/BOOTX64.EFI limine/LICENSE build/efiroot/EFI/BOOT/
	cp misc/limine.conf build/efiroot/boot/
	cp target/x86_64-unknown-none/debug/limine-boot-demo build/efiroot/boot/
	$(CROSS_COMPILE)strip -s -g build/efiroot/boot/limine-boot-demo

.PHONY: kernel
kernel:
	cargo build
