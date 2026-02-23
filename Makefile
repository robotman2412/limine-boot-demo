# Copyright © 2026, __robot@PLT
# SPDX-License-Identifier: MIT

EFI_PART_SIZE ?= 4MiB
QEMU_FLAGS ?= -m 2G
ARCH ?= x86_64
QEMU ?= qemu-system-$(ARCH)
CROSS_COMPILE ?= $(ARCH)-linux-gnu-
GDB ?= $(shell command -v $(CROSS_COMPILE)gdb || echo gdb)
STRIP ?= $(shell command -v $(CROSS_COMPILE)strip || echo strip)
BUILDTYPE ?= debug
BUILDDIR ?= build/$(ARCH)-$(BUILDTYPE)
CARGO_FLAGS ?=

ifeq '$(BUILDTYPE)' 'release'
CARGO_BUILD_FLAGS ?= --release
else
CARGO_BUILD_FLAGS ?=
endif

.PHONY: image
image: sysroot
	mkdir -p build/image
	./scripts/make_fatfs.sh $(EFI_PART_SIZE) $(BUILDDIR)/efiroot $(BUILDDIR)/efi.fatfs
	./scripts/make_image.sh \
		$(BUILDDIR)/image.hdd \
		'EFI partition'  boot $(BUILDDIR)/efi.fatfs 0x0700

.PHONY: qemu
qemu: edk2-ovmf image
	$(QEMU) $(QEMU_FLAGS) -s -accel tcg \
		-drive if=pflash,unit=0,format=raw,file=edk2-ovmf/ovmf-code-$(ARCH).fd \
		-drive format=raw,file=$(BUILDDIR)/image.hdd,cache=none

edk2-ovmf:
	curl -L https://github.com/osdev0/edk2-ovmf-nightly/releases/latest/download/edk2-ovmf.tar.gz | gunzip | tar -xf -

.PHONY: gdb
gdb:
	$(GDB) -x misc/gdbinit $(BUILDDIR)/limine-boot-demo

.PHONY: sysroot
sysroot: kernel
	# EFI root folders
	mkdir -p $(BUILDDIR)/efiroot/EFI/BOOT
	mkdir -p $(BUILDDIR)/efiroot/boot
	
	# Copy the bootloader, modules and kernel into the EFI root
	cp limine/BOOTX64.EFI limine/LICENSE $(BUILDDIR)/efiroot/EFI/BOOT/
	cp misc/limine.conf $(BUILDDIR)/efiroot/boot/
	cp $(BUILDDIR)/limine-boot-demo $(BUILDDIR)/efiroot/boot/
	$(STRIP) -s -g $(BUILDDIR)/efiroot/boot/limine-boot-demo

# Multiple kernel targets needed because of differing target triplets
# Don't call these yourself, you'll break it
.PHONY: kernel
kernel: -helper-kernel-$(ARCH)

.PHONY: -helper-kernel-x86_64
-helper-kernel-x86_64:
	cargo $(CARGO_FLAGS) build $(CARGO_BUILD_FLAGS) --target=x86_64-unknown-none
	mkdir -p $(BUILDDIR)
	cp target/x86_64-unknown-none/$(BUILDTYPE)/limine-boot-demo $(BUILDDIR)/

.PHONY: -helper-kernel-aarch64
-helper-kernel-aarch64:
	cargo $(CARGO_FLAGS) build $(CARGO_BUILD_FLAGS) --target=aarch64-unknown-none-softfloat
	mkdir -p $(BUILDDIR)
	cp target/aarch64-unknown-none-softfloat/$(BUILDTYPE)/limine-boot-demo $(BUILDDIR)/

.PHONY: -helper-kernel-riscv64
-helper-kernel-riscv64:
	cargo $(CARGO_FLAGS) build $(CARGO_BUILD_FLAGS) --target=riscv64imac-unknown-none-elf
	mkdir -p $(BUILDDIR)
	cp target/riscv64imac-unknown-none-elf/$(BUILDTYPE)/limine-boot-demo $(BUILDDIR)/
