# Copyright © 2026, __robot@PLT
# SPDX-License-Identifier: MIT

EFI_PART_SIZE ?= 4MiB
ROOT_PART_SIZE ?= 57MiB
CROSS_COMPILE ?= riscv64-linux-gnu-
QEMU ?= qemu-system-riscv64

.PHONY: image
image: sysroot
	mkdir -p build/image
	./scripts/make_fatfs.sh $(EFI_PART_SIZE) build/efiroot build/image/efi.fatfs
	./scripts/make_e2fs.sh $(ROOT_PART_SIZE) build/sysroot build/image/root.e2fs
	./scripts/make_image.sh \
		build/image.hdd \
		'EFI partition'  boot build/image/efi.fatfs 0x0700 \
		'Root partition' root build/image/root.e2fs 0x8300

.PHONY: qemu
qemu: build/cache/OVMF_RISCV64.fd image
	$(QEMU) -s \
		-M virt,acpi=off -cpu rv64,sv48=false -smp 4 -m 1G \
		-device pcie-root-port,bus=pcie.0,id=pcisw0 \
		-device qemu-xhci,bus=pcisw0 -device usb-kbd \
		-drive if=pflash,unit=0,format=raw,file=build/cache/OVMF_RISCV64.fd \
		-drive if=none,id=hd0,format=raw,file=build/image.hdd,cache=none \
		-device ahci,id=achi0 \
		-device ide-hd,drive=hd0,bus=achi0.0 \
		-serial mon:stdio -nographic \
	| scripts/address-filter.py -L -A $(CROSS_COMPILE)addr2line target/kernel_riscv64/debug/positron

build/cache/OVMF_RISCV64.fd:
	mkdir -p build/cache
	test -f build/cache/OVMF_RISCV64.fd || ( \
		cd build/cache \
		&& curl -o OVMF_RISCV64.fd https://retrage.github.io/edk2-nightly/bin/RELEASERISCV64_VIRT_CODE.fd \
		&& dd if=/dev/zero of=OVMF_RISCV64.fd bs=1 count=0 seek=33554432 \
	)

.PHONY: sysroot
sysroot: kernel
	# EFI root folders
	mkdir -p build/efiroot/EFI/BOOT
	mkdir -p build/efiroot/boot
	
	# Copy the bootloader, modules and kernel into the EFI root
	cp limine/BOOTRISCV64.EFI limine/LICENSE build/efiroot/EFI/BOOT/
	cp misc/limine.conf build/efiroot/boot/
	cp target/kernel_riscv64/debug/positron build/efiroot/boot/
	$(CROSS_COMPILE)strip -s -g build/efiroot/boot/positron
	
	# System root folders
	mkdir -p build/sysroot/boot
	mkdir -p build/sysroot/dev
	mkdir -p build/sysroot/tmp
	mkdir -p build/sysroot/mnt
	
	# Copy some dummy things into the system root
	echo "This is a text file, ok." > build/sysroot/test.txt

.PHONY: kernel
kernel:
	cargo build
