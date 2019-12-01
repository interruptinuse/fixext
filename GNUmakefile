ROOT_DIR   = $(patsubst %/,%,$(dir $(realpath $(firstword $(MAKEFILE_LIST)))))
VENDOR_DIR = $(ROOT_DIR)/vendor
VERSION    = $(shell grep -m1 '^version' Cargo.toml | grep -Po '(?<=")\d+\.\d+\.\d+(?=")')
WINAPI     = win64
WINDIST    = fixext-$(WINAPI)-$(VERSION)
ARCH       = x86_64
MINGW_TGT  = $(ARCH)-w64-mingw32
CARGO_TGT  = $(ARCH)-pc-windows-gnu

LIBGNURX = libgnurx-0.dll
LIBMAGIC = libmagic-1.dll
PTHREAD  = pthreadGC2.dll
DLLS     = $(LIBGNURX) $(LIBMAGIC) $(PTHREAD)

GAWK = gawk
PERL = perl
SED  = sed



.PHONY: list
list:
	@echo "The following targets are available:"
	@echo "  $(MAKE) list"
	@$(MAKE) -pRrq -f $(lastword $(MAKEFILE_LIST)) : 2>/dev/null\
	  | $(GAWK) -v RS= -F: '/^# File/,/^# Finished Make data base/ {if ($$1 !~ "^[#.]") {print $$1}}'\
	  | sort | egrep -v -e '^[^[:alnum:]]' -e '^$@$$' | $(SED) 's/^/  $(MAKE) /g'

	@echo

	@echo "The following variables and their defaults are available:"
	@$(MAKE) -pRrq -f $(lastword $(MAKEFILE_LIST)) : 2>/dev/null\
	  | $(GAWK) '/^# Variables/,/^# Directories/' \
	  | $(GAWK) 'match($$0, /^# makefile .*line ([[:digit:]]+)\)$$/, m) {getline; print m[1], $$0}' \
	  | sort -nru -k1,1 \
	  | cut -d' ' -f2- \
	  | tac \
	  | $(GAWK) 'END {print max} {l=length($$1);max=(max>l?max:l);print}' \
	  | tac \
	  | $(PERL) -ale 'BEGIN{$$,=" ";$$a=int(<STDIN>)}print"@{[sprintf(\"%-$${a}s\",@F[0]),@F[1..$$#F]]}"' \
	  | tac \
	  | $(SED) 's,^,  ,g'

.PHONY: dist
dist: $(WINDIST).zip

$(WINDIST).zip: fixext.exe fixext.1.html magic.mgc $(DLLS)
	rm -rf $(WINDIST) $(WINDIST).zip
	mkdir -p $(WINDIST)
	cp $(filter-out magic.mgc,$^) $(WINDIST)
	zip -9 -r $(WINDIST).zip $(WINDIST)


.PHONY: update
update:
	git submodule update


fixext.exe: $(DLLS) magic.mgc
	cross build --target $(CARGO_TGT) --release
	cp target/$(CARGO_TGT)/release/fixext.exe $(ROOT_DIR)/fixext.exe


$(LIBGNURX):
	cd $(VENDOR_DIR)/mingw-libgnurx \
	  && ./configure --host=$(MINGW_TGT) \
	  && make clean && make \
	  && cp libgnurx-0.dll  $(ROOT_DIR)/$@


$(LIBMAGIC) magic.mgc: $(LIBGNURX)
	cd $(VENDOR_DIR)/file && autoreconf -fvi
	cd $(VENDOR_DIR)/file && ./configure && make clean && make
	cp $(VENDOR_DIR)/file/magic/magic.mgc       $(ROOT_DIR)/magic.mgc
	cd $(VENDOR_DIR)/file && ./configure --host=$(MINGW_TGT) \
	  LDFLAGS=-L$(VENDOR_DIR)/mingw-libgnurx CFLAGS=-I$(VENDOR_DIR)/mingw-libgnurx
	cd $(VENDOR_DIR)/file && make clean && make -C src libmagic.la

	# We need libmagic.dll (without the "-1") for linking, apparently.
	cp $(VENDOR_DIR)/file/src/.libs/libmagic-1.dll  $(ROOT_DIR)/libmagic.dll
	cp $(VENDOR_DIR)/file/src/.libs/libmagic-1.dll  $(ROOT_DIR)/$(LIBMAGIC)


$(PTHREAD): vendor/pthreadGC2.dll
	cp $^ $@


fixext.1.html: fixext.1
	env TZ=UTC groff -mandoc -Thtml <$< >$@

.PHONY: clean
clean:
	rm -f fixext.exe
	rm -f fixext.1.html
	rm -rf $(WINDIST) $(WINDIST).zip
	rm -f $(addprefix $(ROOT_DIR)/,magic.mgc $(DLLS) libmagic.dll)
	git submodule foreach find . -delete
	git submodule update
