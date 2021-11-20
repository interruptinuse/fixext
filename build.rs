use std::collections::HashSet;
use std::path;
use std::env;
use std::fs;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::path::PathBuf;
use std::vec::Vec;
use std::process::Command;

use glob::glob;

use regex::Regex;

use winres::WindowsResource;


fn main() {
  let target = env::var("TARGET").unwrap();
  let target_arch = target.split('-').next().unwrap();
  let windows = target.contains("windows");

  let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
  let desc_types_cbor_file = out_dir.join("desc.types.cbor");
  let mime_types_cbor_file = out_dir.join("mime.types.cbor");

  if windows {
    let magic_file = out_dir.join("magic.mgc");
    let magic_file_src = "vendor/build/magic.mgc".to_string();
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

      let splits: Vec<String> = (*l).split('\t').map(|s| s.to_string()).collect();

      let regex = splits[0].to_string();
      let exts  = (&splits[1..]).to_vec();

      eprintln!("Description: {}, exts: {:?}", regex, exts);

      if Regex::new(&*regex).is_err() {
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

      let splits: Vec<String> = (*l).split(' ').map(|s| s.to_string()).collect();

      let mime = splits[0].to_string();
      let exts = (&splits[1..]).to_vec();

      eprintln!("MIME: {}, exts: {:?}", mime, exts);

      if mime.is_empty() {
        eprintln!("MIME is empty, skipping");
        continue;
      }

      if exts.is_empty() {
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

    let exe_name = "fixext.exe";

    let (tool_path, tool_windres, tool_ar) = {
      fn path_to_dir_base(s: &path::Path) -> (String, String) {
        use std::path::Component::*;
        let components = s.components().collect::<Vec<path::Component>>();

        let dirname = {
          if components.len() == 1 {
            match components[0] {
              CurDir                => components[0],
              ParentDir | Normal(_) => path::Component::CurDir,
              _                     => path::Component::RootDir,
            }.as_os_str().to_string_lossy().to_string()
          } else {
            let mut pb = PathBuf::new();

            for s in &components[0..components.len()-1] {
              pb.push(s);
            }

            String::from(pb.as_os_str().to_string_lossy())
          }
        };

        let basename = {
          if components.len() == 1 {
            match components[0] {
              CurDir | ParentDir | Normal(_) => {
                let mut pb = PathBuf::new();
                pb.push(path::Component::CurDir);
                pb.push(components[0]);
                pb
              },
              _ => {
                let mut pb = PathBuf::new();
                pb.push(path::Component::RootDir);
                pb
              },
            }
          } else {
            let mut pb = PathBuf::new();
            pb.push(path::Component::CurDir);
            pb.push(components[components.len()-1]);
            pb
          }
        }.as_os_str().to_string_lossy().to_string();

        (dirname, basename)
      }


      let tool_gcc = env::var("MINGW_GCC")
        .unwrap_or(format!("{}-w64-mingw32-gcc", target_arch));

      let get_tool_path = |t: &str| {
        let mut p = String::from_utf8(
          Command::new(&tool_gcc)
            .arg(format!("-print-prog-name={}", t))
            .output()
            .expect(&*format!("Failed to execute {} -print-prog-name={}", tool_gcc, t))
            .stdout)
          .expect(
            &*format!(
              "{} -print-prog-name={} did not return valid UTF-8",
              tool_gcc,
              t)
          );

        p.pop();

        let (d, b) = path_to_dir_base(Path::new(&p));
        (p, d, b)
      };

      let (w, w_d, w_b) = get_tool_path("windres");
      let (a, a_d, a_b) = get_tool_path("ar");

      assert!((w_d == a_d), "{} dirname is not the same as dirname {}", w, a);

      (w_d, w_b, a_b)
    };


    let mut res = WindowsResource::new();
    res.set_icon("asset/wrench-pencil.ico")
       .set_language(0x0409) // MAKELANGID(LANG_ENGLISH, SUBLANG_ENGLISH_US)
       .set("InternalName", exe_name)
       .set_toolkit_path(&tool_path)
       .set_windres_path(&tool_windres)
       .set_ar_path(&tool_ar);
    res.compile().expect("res.compile() failed");
  }
}
