  fixext
========
  detect file type by content and fix incorrect file extensions
---------------------------------------------------------------
  This utility uses `libmagic(3)` and MIME-to-extension mapping file from the
  Apache webserver (mime.types) to detect files with incorrect extensions, and
  rename them.
  ```
  $ file --mime-type *
  binary-file.txt: application/octet-stream
  jpeg.png:        image/jpeg
  mp4.mkv:         video/mp4
  $ fixext *
  binary-file.txt -> binary-file.bin
  jpeg.png -> jpeg.jpg
  mp4.mkv -> mp4.mp4
  ```
  For more information, try `fixext --help` or `man ./fixext.1`.
