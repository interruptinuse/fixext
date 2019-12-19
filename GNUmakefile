THIS    := $(realpath $(firstword $(MAKEFILE_LIST)))
ROOT    := $(patsubst %/,%,$(dir $(THIS)))
VENDOR  := vendor
BUILD   := $(VENDOR)/build
VERSION := $(shell git describe --tags --dirty | cut -c2-)
WINDIST := fixext-win-$(VERSION)
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
DISTFILES     := fixext32.exe fixext.exe fixext.1.html


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


define create-exe-target
$1: $$(addprefix $$(BUILD)/$2/,$$(LIBS)) $$(BUILD)/magic.mgc
	$$(eval CARGO_TGT = $2-pc-windows-gnu)
	$$(CARGO) build --target $$(CARGO_TGT) --release
	cp target/$$(CARGO_TGT)/release/fixext.exe $1
endef


$(eval $(call create-exe-target,fixext32.exe,i686))
$(eval $(call create-exe-target,fixext.exe,x86_64))


$(VENDOR)/file/configure: | $(VENDOR)/file/configure.ac
	cd $(VENDOR)/file \
	  && autoreconf -fvi


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


define create-vendored-deps
$$(BUILD)/$1/winpthreads $$(BUILD)/$1/mingw-libgnurx $$(BUILD)/$1/file:
	mkdir -p $$@

.ONESHELL: $$(addprefix $$(BUILD)/$1/,$$(LIBS))

$$(BUILD)/$1/$$(LIBWINPTHREAD): | $$(BUILD)/$1/winpthreads
	set -e
	cd $$(BUILD)/$1/winpthreads
	../../../winpthreads/configure --host=$1-w64-mingw32 \
	  --enable-static --disable-shared
	$$(MAKE) clean
	$$(MAKE)
	cp .libs/$$(LIBWINPTHREAD) ..
	$1-w64-mingw32-objcopy -R.rsrc ../$$(LIBWINPTHREAD)

$$(BUILD)/$1/$$(LIBGNURX): $$(BUILD)/$1/$$(LIBWINPTHREAD) | $$(BUILD)/$1/mingw-libgnurx
	set -e
	cd $$(BUILD)/$1/mingw-libgnurx
	$1-w64-mingw32-gcc -c -o regex.o -I$$(ROOT)/$$(VENDOR)/mingw-libgnurx $$(CFLAGS) \
	  $$(ROOT)/$$(VENDOR)/mingw-libgnurx/regex.c -Wl,-Bstatic -L.. -lwinpthread \
	  -static-libgcc -static-libstdc++
	$1-w64-mingw32-ar rcs -o $$(LIBGNURX) regex.o
	cp $$(LIBGNURX) ..
	$1-w64-mingw32-objcopy -R.rsrc ../$$(LIBGNURX)

$$(BUILD)/$1/$$(LIBMAGIC): $$(VENDOR)/file/configure \
                             $$(addprefix $$(BUILD)/$1/,$$(LIBWINPTHREAD) $$(LIBGNURX)) \
                             | $$(BUILD)/$1/file
	set -e
	mkdir -p $$(BUILD)/$1/file
	cd $$(BUILD)/$1/file
	export LDFLAGS="-Wl,-Bdynamic -lshlwapi -Wl,-Bstatic -static-libgcc \
	  -static-libstdc++ -L.. -lwinpthread"
	export CFLAGS="-I$$(ROOT)/$$(VENDOR)/mingw-libgnurx"
	../../../file/configure --host=$1-w64-mingw32 --enable-static --disable-shared
	$$(MAKE) clean
	$$(MAKE) -C src magic.h
	$$(MAKE) -C src libmagic.la
	cp src/.libs/$$(LIBMAGIC) ..
	$1-w64-mingw32-objcopy -R.rsrc ../$$(LIBMAGIC)
endef


$(eval $(call create-vendored-deps,i686))
$(eval $(call create-vendored-deps,x86_64))
