THIS    := $(realpath $(firstword $(MAKEFILE_LIST)))
ROOT    := $(patsubst %/,%,$(dir $(THIS)))
VENDOR  := vendor
BUILD   := $(VENDOR)/build
ARCH    := i686
VERSION := $(shell git describe --tags --dirty | cut -c2-)
WINDIST := fixext-win32-$(VERSION)
CARGO   := cargo

BASH  = bash
GAWK  = gawk
PERL  = perl
SED   = sed
ZIP   = zip
GROFF = groff


LIBWINPTHREAD := libwinpthread.a
LIBGNURX      := libgnurx.a
LIBMAGIC      := libmagic.a
LIBS          := $(LIBWINPTHREAD) $(LIBGNURX) $(LIBMAGIC)
LIBDIRS       := winpthreads mingw-libgnurx file
DISTFILES     := fixext.exe fixext.1.html


TZ := UTC
export TZ


.PHONY: warn
warn:
	$(warning You're attempting to cross-compile fixext for Windows.  Did you mean 'cargo build'?)
	$(warning (run 'make dist' instead if you know what you're doing.))
	$(error Misguided command)


.PHONY: dist win
dist win $(WINDIST).zip: $(addprefix $(WINDIST)/,$(DISTFILES))
	$(ZIP) -r9 $(WINDIST).zip $(WINDIST)


$(WINDIST)/%: %
	mkdir -p $(WINDIST)
	cp $^ $@


.PHONY: clean
clean:
	rm -f $(DISTFILES) $(WINDIST).zip
	rm -rf $(WINDIST) $(VENDOR)/build
	$(CARGO) clean


.PHONY: clean-submodules
clean-submodules:
	git submodule foreach find . -delete
	git submodule update


.PHONY: clean-all
clean-all: clean clean-submodules


fixext.1.html: fixext.1
	env TZ=UTC $(GROFF) -mandoc -Thtml <$< >$@


fixext.exe: $(addprefix $(BUILD)/$(ARCH)/,$(LIBS)) $(BUILD)/magic.mgc
	WINDRES=$(ARCH)-w64-mingw32-windres AR=$(ARCH)-w64-mingw32-ar $(CARGO) build --target $(ARCH)-pc-windows-gnu --release
	cp target/$(ARCH)-pc-windows-gnu/release/fixext.exe $@


$(VENDOR)/file/configure: | $(VENDOR)/file/configure.ac
	cd $(VENDOR)/file \
	  && libtoolize && autoreconf -fvi


$(VENDOR)/file/configure.ac:
	git submodule update $(VENDOR)/file


.ONESHELL: $(BUILD)/magic.mgc
$(BUILD)/magic.mgc: $(VENDOR)/file/configure
	set -e
	mkdir -p $(BUILD)/magic
	cd $(BUILD)/magic
	../../file/configure
	$(MAKE) clean
	$(MAKE) -C src
	$(MAKE) -C magic FILE_COMPILE="../src/file" magic.mgc
	cp magic/magic.mgc ..


$(BUILD)/$(ARCH)/winpthreads $(BUILD)/$(ARCH)/mingw-libgnurx $(BUILD)/$(ARCH)/file:
	mkdir -p $@

.ONESHELL: $(addprefix $(BUILD)/$(ARCH)/,$(LIBS))

$(BUILD)/$(ARCH)/$(LIBWINPTHREAD): | $(BUILD)/$(ARCH)/winpthreads
	set -e
	cd $(BUILD)/$(ARCH)/winpthreads
	../../../winpthreads/configure --host=$(ARCH)-w64-mingw32 \
	  --enable-static --disable-shared
	$(MAKE) clean
	$(MAKE)
	cp .libs/$(LIBWINPTHREAD) ..

$(BUILD)/$(ARCH)/$(LIBGNURX): $(BUILD)/$(ARCH)/$(LIBWINPTHREAD) | $(BUILD)/$(ARCH)/mingw-libgnurx
	set -e
	cd $(BUILD)/$(ARCH)/mingw-libgnurx
	$(ARCH)-w64-mingw32-gcc -c -o regex.o -I$(ROOT)/$(VENDOR)/mingw-libgnurx $(CFLAGS) \
	  $(ROOT)/$(VENDOR)/mingw-libgnurx/regex.c -Wl,-Bstatic -L.. -lwinpthread \
	  -static-libgcc -static-libstdc++
	$(ARCH)-w64-mingw32-ar rcs -o $(LIBGNURX) regex.o
	cp $(LIBGNURX) ..

$(BUILD)/$(ARCH)/$(LIBMAGIC): $(VENDOR)/file/configure \
                             $(addprefix $(BUILD)/$(ARCH)/,$(LIBWINPTHREAD) $(LIBGNURX)) \
                             | $(BUILD)/$(ARCH)/file
	set -e
	mkdir -p $(BUILD)/$(ARCH)/file
	cd $(BUILD)/$(ARCH)/file
	export LDFLAGS="-Wl,-Bdynamic -lshlwapi -Wl,-Bstatic -static-libgcc \
	  -static-libstdc++ -L.. -lwinpthread"
	export CFLAGS="-I$(ROOT)/$(VENDOR)/mingw-libgnurx"
	../../../file/configure --host=$(ARCH)-w64-mingw32 --enable-static --disable-shared
	$(MAKE) clean
	$(MAKE) -C src magic.h
	$(MAKE) -C src libmagic.la
	cp src/.libs/$(LIBMAGIC) ..
