# Makefile – build a single kernel module called binfmt_wasm.ko
obj-m := binfmt_wasm.o

# Location of the *running* kernel’s build tree
KDIR  := /lib/modules/$(shell uname -r)/build
PWD   := $(shell pwd)
CC := /usr/bin/x86_64-linux-gnu-gcc-13

WASM_RUNTIME_LIBS := -lwasmtime
WASM_ARTIFACTS := $(HOME)/wasmtime/artifacts
WASMTIME_PATH := $(HOME)/wasmtime
CFLAGS  := -O2 -Wall

.PHONY: all module runner clean


runner: wasm_launcher.c
	$(CC) wasm_launcher.c -I "$(WASMTIME_PATH)/crates/c-api/include" "$(WASMTIME_PATH)/target/release/libwasmtime.a" -lpthread -ldl -lm -o wasm-launcher

module:
	$(MAKE) -C $(KDIR) M=$(PWD) CC="$(CC)" modules

all: module runner


clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	$(RM) -f wasm-launcher

