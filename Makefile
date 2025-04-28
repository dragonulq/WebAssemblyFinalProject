# Makefile – build a single kernel module called binfmt_wasm.ko
obj-m := binfmt_wasm.o

# Location of the *running* kernel’s build tree
KDIR  := /lib/modules/$(shell uname -r)/build
PWD   := $(shell pwd)

all:
	$(MAKE) -C $(KDIR) M=$(PWD) CC=/usr/bin/x86_64-linux-gnu-gcc-13 modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
