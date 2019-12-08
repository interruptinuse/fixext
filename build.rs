use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::vec::Vec;

use glob::glob;

use regex::Regex;



fn main() {
  let target = env::var("TARGET").unwrap();
  let target_arch = target.split('-').nth(0).unwrap();
  let windows = target.contains("windows");

  let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
  let desc_types_cbor_file = out_dir.join("desc.types.cbor");
  let mime_types_cbor_file = out_dir.join("mime.types.cbor");

  if windows {
    let magic_file = out_dir.join("magic.mgc");
    let magic_file_src = format!("vendor/build/{}/magic.mgc", target_arch);
    println!("rerun-if-changed={}", magic_file.to_string_lossy());
    fs::copy(magic_file_src, magic_file).expect("could not find magic.mgc (run `./windist.sh`)");
  }

  println!("rerun-if-changed=data");

  let data_dir = Path::new("./data");
  let data_files = fs::read_dir(data_dir).unwrap();

  for f in data_files {
    let name = f.unwrap().file_name().into_string().unwrap();
    println!("rerun-if-changed=data/{}", name);
  }

  println!("rerun-if-changed={}", desc_types_cbor_file.to_string_lossy());
  println!("rerun-if-changed={}", mime_types_cbor_file.to_string_lossy());

  let mut desc_types_set: HashSet<String> = HashSet::new();
  let mut desc_types: Vec<(String, Vec<String>)> = vec![];

  for dtf in glob("data/*.desc.types").unwrap() {
    let p = match dtf {
      Ok(path) => path,
      e        => panic!("glob returned an error: {:?}", e)
    };

    let f = fs::File::open(p).unwrap();
    let r = io::BufReader::new(f);

    for line in r.lines() {
      let line = line.unwrap();
      let l = line.trim_start().trim_end();

      let comment_r = Regex::new(r"^\s*#").unwrap();
      let empty_r   = Regex::new(r"^\s*$").unwrap();

      eprintln!("desc.types line: {}", l);

      if empty_r.is_match(l) || comment_r.is_match(l) {
        eprintln!("(line is empty or a comment, skipping)");
        continue;
      }

      let splits: Vec<String> = l.clone().split("\t").map(|s| s.to_string()).collect();

      let regex = splits[0].to_string();
      let exts  = (&splits[1..]).to_vec();

      eprintln!("Description: {}, exts: {:?}", regex, exts);

      if let Err(_) = Regex::new(&*regex) {
        eprintln!("Description is an invalid regex, skipping");
        continue;
      }

      if desc_types_set.contains(&regex) {
        eprintln!("Description already processed, skipping");
        continue;
      }

      desc_types_set.insert(regex.clone());

      desc_types.push((regex, exts));
    }
  }

  let desc_types_cbor = fs::File::create(desc_types_cbor_file).unwrap();
  serde_cbor::to_writer(desc_types_cbor, &desc_types).unwrap();

  let mut mime_types_set: HashSet<String> = HashSet::new();
  let mut mime_types: Vec<(String, Vec<String>)> = vec![];

  for mtf in glob("data/*.mime.types").unwrap() {
    let p = match mtf {
      Ok(path) => path,
      e        => panic!("glob returned an error: {:?}", e)
    };

    let f = fs::File::open(p).unwrap();
    let r = io::BufReader::new(f);

    for line in r.lines() {
      let line = line.unwrap();
      let l = line.trim_start().trim_end();

      let comment_r = Regex::new(r"^\s*#").unwrap();
      let empty_r   = Regex::new(r"^\s*$").unwrap();

      eprintln!("mime.types line: {}", l);

      if empty_r.is_match(l) || comment_r.is_match(l) {
        eprintln!("(line is empty or a comment, skipping)");
        continue;
      }

      let splits: Vec<String> = l.clone().split(" ").map(|s| s.to_string()).collect();

      let mime = splits[0].to_string();
      let exts = (&splits[1..]).to_vec();

      eprintln!("MIME: {}, exts: {:?}", mime, exts);

      if mime.is_empty() {
        eprintln!("MIME is empty, skipping");
        continue;
      }

      if exts.len() == 0 {
        eprintln!("MIME is associated with no extensions, skipping");
        continue;
      }

      if mime_types_set.contains(&mime) {
        eprintln!("MIME has already been processed, skipping: {}", mime);
        continue;
      }

      mime_types_set.insert(mime.clone());

      mime_types.push((mime, exts));
    }
  }

  let mime_types_cbor = fs::File::create(mime_types_cbor_file).unwrap();
  serde_cbor::to_writer(mime_types_cbor, &mime_types).unwrap();

  if windows {
    println!("cargo:rustc-link-search=native=vendor/build/{}/", target_arch);
    println!("cargo:rustc-link-lib=static=magic");
    println!("cargo:rustc-link-lib=static=gnurx");
    println!("cargo:rustc-link-lib=static=winpthread");
    println!("cargo:rustc-link-lib=msvcrt");
    println!("cargo:rustc-link-lib=shlwapi");
  }

  return Ok(());
}
