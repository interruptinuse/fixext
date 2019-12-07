#!/usr/bin/env bash

set -ex

export TZ=UTC

VERSION="$(git describe --tags --dirty | cut -c2-)"
WINDIST="fixext-win-$VERSION"
{ [[ ! -e $WINDIST ]] && [[ ! -e $WINDIST.zip ]] ; } || false

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
cd "$ROOT_DIR"


rm -fv fixext.exe fixext32.exe fixext.1.html "$WINDIST".zip
rm -rfv "$WINDIST"
git submodule foreach find . -delete
git submodule update


pushd vendor/file
autoreconf -fvi
popd


for ARCH in i686 x86_64 ; do
  MINGW_TARGET=$ARCH-w64-mingw32
  CARGO_TARGET=$ARCH-pc-windows-gnu

  rm -rf   vendor/build/$ARCH
  mkdir -p vendor/build/$ARCH

  cd vendor/build/$ARCH

  mkdir -p winpthreads
  pushd winpthreads
  "$ROOT_DIR"/vendor/winpthreads/configure --host=$MINGW_TARGET --enable-static --disable-shared
  make clean
  make
  cp .libs/libwinpthread.a ..
  popd
  pwd

  mkdir -p mingw-libgnurx
  pushd mingw-libgnurx
  # shellcheck disable=SC2086
	$ARCH-w64-mingw32-gcc -c -o regex.o \
    -I"$ROOT_DIR"/vendor/mingw-libgnurx $CFLAGS "$ROOT_DIR"/vendor/mingw-libgnurx/regex.c \
	  -Wl,-Bstatic -L.. -lwinpthread -static-libgcc -static-libstdc++
	$ARCH-w64-mingw32-ar rcs -o libgnurx.a regex.o
	cp libgnurx.a ..
  popd
  pwd

  mkdir -p file
  pushd file
  export LDFLAGS="-L$ROOT_DIR/vendor/build/$ARCH -Wl,-Bdynamic -lshlwapi \
  -Wl,-Bstatic -static-libgcc -static-libstdc++ -lwinpthread"
  export CFLAGS="-I$ROOT_DIR/vendor/mingw-libgnurx"
  "$ROOT_DIR"/vendor/file/configure --host=$MINGW_TARGET --enable-static --disable-shared
  make clean
  make -C src magic.h
  make -C src libmagic.la
  cp src/.libs/libmagic.a ..

  make -C src file.exe
  make -C magic FILE_COMPILE="wine ../src/file.exe" magic.mgc
  cp magic/magic.mgc ..
  popd
  pwd

  $ARCH-w64-mingw32-strip -g ./*.a

  cd "$ROOT_DIR"
  ${CARGO-cargo} build -vv --target $CARGO_TARGET --release
done


groff -mandoc -Thtml <fixext.1 >fixext.1.html

cp target/x86_64-pc-windows-gnu/release/fixext.exe fixext.exe
cp target/i686-pc-windows-gnu/release/fixext.exe fixext32.exe


mkdir -p "$WINDIST"
cp fixext.1.html fixext.exe fixext32.exe "$WINDIST"
zip -r9 "$WINDIST".zip "$WINDIST"
rm -rfv fixext.1.html fixext.exe fixext32.exe "$WINDIST"
