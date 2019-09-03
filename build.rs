use std::error::Error;
use std::io;
use std::io::BufRead;
use std::fs;
use std::path::Path;
use std::vec::Vec;
use std::collections::HashSet;

extern crate serde;

extern crate serde_cbor;

extern crate glob;
use glob::glob;

extern crate regex;
use regex::Regex;


fn main() -> Result<(), Box<dyn Error>>  {
  println!("rerun-if-changed=data");

  let data_dir = Path::new("./data");
  let data_files = fs::read_dir(data_dir).unwrap();

  for f in data_files {
    let name = f.unwrap().file_name().into_string().unwrap();
    println!("rerun-if-changed=data/{}", name);
  }

  println!("rerun-if-changed=desc.types.cbor");
  println!("rerun-if-changed=mime.types.cbor");


  let mut desc_types_set: HashSet<String> = HashSet::new();
  let mut desc_types: Vec<(String, Vec<String>)> = vec![];

  for /* desc.types file */ dtf in glob("data/*.desc.types").unwrap() {
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

  let desc_types_cbor = fs::File::create("desc.types.cbor").unwrap();
  serde_cbor::to_writer(desc_types_cbor, &desc_types)?;


  let mut mime_types_set: HashSet<String> = HashSet::new();
  let mut mime_types: Vec<(String,Vec<String>)> = vec![];

  for /* mime.types file */ mtf in glob("data/*.mime.types").unwrap() {
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

  let mime_types_cbor = fs::File::create("mime.types.cbor").unwrap();
  serde_cbor::to_writer(mime_types_cbor, &mime_types)?;


  return Ok(());
}
