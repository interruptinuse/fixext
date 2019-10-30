#![feature(label_break_value)]

#![allow(clippy::needless_return)]
#![allow(clippy::cognitive_complexity)]


const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const AUTHORS: Option<&'static str> = option_env!("CARGO_PKG_AUTHORS");
const DESCRIP: Option<&'static str> = option_env!("CARGO_PKG_DESCRIPTION");

const MIME_TYPES_CBOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/mime.types.cbor"));
const DESC_TYPES_CBOR: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/desc.types.cbor"));

#[cfg(not(windows))]
const DEFAULT_MGC: &str = "/usr/share/misc/magic.mgc";
#[cfg(not(windows))]
const BUILTIN_MGC: &[u8] = &[];

#[cfg(windows)]
const DEFAULT_MGC: &str = "";
#[cfg(windows)]
const BUILTIN_MGC: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/magic.mgc"));

use std::fs;
use std::process;
use std::convert::TryInto;
use std::path;
use std::path::PathBuf;
use std::path::Component::*;
use std::vec::Vec;
use std::clone::Clone;
use std::collections::HashMap;

use magic::CookieFlags;
use magic::flags::MIME_TYPE;

use regex::Regex;

extern crate ansi_term;
use ansi_term::Style;
use ansi_term::ANSIString;

use clap::clap_app;

use rustyline::error::ReadlineError;
use rustyline::Editor;



struct Cookie {
  desc: magic::Cookie,
  mime: magic::Cookie,
}

#[derive(Debug)]
enum MagicMatch {
  Description(Regex, Vec<String>),
  MIME(String, Vec<String>),
  None
}

#[derive(Debug)]
enum MagicDatabase<'a> {
  File(&'a str),
  Buffer(&'a [u8])
}

#[derive(Debug, Default)]
struct Opts {
  dry:         bool,
  interactive: bool,
  force:       bool,
  recursive:   bool,
  append:      bool,
  detect:      bool,
  dump:        bool,
  nobuiltin:   bool,
  matchinfo:   bool,
  magicfile:   Option<String>,
  extdot:      i32,
  verbose:     bool,
}

struct Types {
  desc: Vec<(Regex, Vec<String>)>,
  mime: HashMap<String, Vec<String>>,
}



fn bold(s: &str) -> ANSIString<> {
  if cfg!(not(windows)) {
    return Style::default().bold().paint(s);
  }
  else {
    return Style::default().paint(s);
  }
}


fn path_to_dir_base(s: &path::Path) -> (String, String) {
  let components = s.components().collect::<Vec<path::Component>>();

  let dirname = {
    if components.len() == 1 {
      match components[0] {
        CurDir                => components[0],
        ParentDir | Normal(_) => path::Component::CurDir,
        _                     => path::Component::RootDir,
      }.as_os_str().to_string_lossy().to_string()
    }
    else {
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
        CurDir | ParentDir | Normal(_) => components[0],
        _                              => path::Component::RootDir,
      }
    }
    else {
      components[components.len()-1]
    }
  }.as_os_str().to_string_lossy().to_string();

  (dirname, basename)
}


fn magic_load(cookie: &Cookie, db: MagicDatabase) {
  #[allow(unused_macros)]
  macro_rules! cook {
    ($member:ident, $method:ident, $arg:ident, $error:expr) => (
      cookie.$member.$method(&[&$arg]).expect(&*format!(
        "Failed to initialize: {}: {}", stringify!($member), $error));
    );
  };

  #[allow(unused_macros)]
  macro_rules! load_both {
    ($method:ident, $arg:ident, $error:expr) => (
      cook!(desc, $method, $arg, $error);
      cook!(mime, $method, $arg, $error);
    );
  };

  match db {
    MagicDatabase::File(s)   => { load_both!(load,         s,
                                    format!("Invalid magic database: {}", s)); },
    MagicDatabase::Buffer(b) => { load_both!(load_buffers, b,
                                    "Invalid built-in magic database"); },
  };
}


fn vec_si<T>(v: &[T], i: i32) -> Option<&[T]> {
  let len_i32: i32 = v.len().try_into().unwrap();

  let idx: usize = if i < 0 {
    if -i > len_i32 {
      return None;
    };

    len_i32 + i
  }
  else {
    if i >= len_i32 {
      return None;
    };

    i
  }.try_into().unwrap();

  return Some(&v[idx..]);
}


fn visit_tree<OkT>(
  t:  &PathBuf,
  fv: &dyn Fn(PathBuf) -> Result<OkT,String>,
  dv: &dyn Fn(PathBuf) -> Result<OkT,String>,
  ev: &dyn Fn(PathBuf, String))
where OkT:      Clone,
{
  let metadata_result = fs::metadata(&t);

  if let Err(e) = metadata_result {
    let estr = e.to_string();
    (ev)(t.clone(), estr.clone());
    return;
  }

  let metadata = metadata_result.unwrap();

  if metadata.is_dir() {
    let dir_result = (dv)(t.clone());

    if dir_result.is_err() {
      return;
    }

    let rd = fs::read_dir(t);

    if let Err(e) = rd {
      let estr = e.to_string();
      (ev)(t.clone(), estr.clone());
      return;
    }

    let rd = rd.unwrap();

    for entry in rd {
      match entry {
        Err(e) => {
          let estr = e.to_string();
          (ev)(t.clone(), estr.clone());
        },

        Ok(de) => {
          visit_tree(&de.path(), fv, dv, ev);
        }
      }
    }
  } else {
    let _ = (fv)(t.clone());
  };
}


fn quote_filename(filename: &str) -> String {
  if cfg!(not(windows)) {
    return shellwords::escape(filename);
  }
  else if filename.contains(' ') {
    String::from("\"") + filename + "\""
  }
  else {
    filename.to_string()
  }
}



fn main() {
  let app = clap::clap_app!(fixext =>
    (version: VERSION.unwrap_or("VERSION"))
    (author:  AUTHORS.unwrap_or("AUTHOR"))
    (about:   DESCRIP.unwrap_or("DESCRIPTION"))
    (@arg FILE: ... required_unless[dump]
                          "Files to check/rename")
    (@arg dry:         -n ... "Dry run: do not actually rename FILEs")
    (@arg interactive: -i ... "Prompt before renaming files")
    (@arg force:       -f ... "When non-interactive, overwrite existing destinations")
    (@arg recursive:   -r ... "Recurse into directory FILEs instead of ignoring")
    (@arg append:      -A ... "Append the correct extension instead of replacing")
    (@arg nobuiltin:   -B ... "Do not use built-in extension associations")
    (@arg detect:      -F ... group("action")
                              "Only print detected types (like `file --mime-type`)")
    (@arg dump:        -D ... group("action")
                              "Print known descriptions/MIME types and associated extensions")
    (@arg matchinfo:   -I ... group("action")
                              "Output null-separated match info")
    (@arg magicfile:   -M [MGC]
                          !empty_values +allow_hyphen_values
                              "Load magic definitions from MGC")
    (@arg extdot:      -L [IDX]
                          !empty_values +allow_hyphen_values
      { |optarg| match optarg.parse::<i32>() {
          Ok(_)  => Ok(()),
          Err(_) => Err(format!("Not an integer: {}", optarg))
        }
      }
      "Cut off the extension after the IDX-th dot.")
    (@arg ovdesc:      -Z [DESC_OVERRIDE] ... number_of_values(1)
                          !empty_values
      "(in form TYPE=EXTS) Override EXTS for files matching description TYPE")
    (@arg ovmime:      -X [MIME_OVERRIDE] ... number_of_values(1)
                          !empty_values
      "(in form MIME=EXTS) Override EXTS for files matching MIME")
    (@arg verbose:     -v --verbose
       "Show additional information about matched file magic"))
    .setting(clap::AppSettings::DeriveDisplayOrder);

  let matches = app.get_matches();
  let files = matches.values_of("FILE").unwrap_or_default();


  let o: Opts = {
    let mut o: Opts = Default::default();

    macro_rules! get_flag {
      ($var:ident) => (o.$var = matches.is_present(stringify!($var)););
    }

    get_flag!(dry);
    get_flag!(interactive);
    get_flag!(force);
    get_flag!(recursive);
    get_flag!(append);
    get_flag!(nobuiltin);
    get_flag!(detect);
    get_flag!(dump);
    get_flag!(matchinfo);
    get_flag!(verbose);

    o.extdot = match matches.value_of("extdot") {
      Some(v)  => v.parse::<i32>().unwrap(),
      None     => -1
    };

    o.magicfile = match matches.value_of("magicfile") {
      Some(path) => Some(String::from(path)),
      None       => None
    };

    o
  };


  let (builtin_desc_types, builtin_mime_types_vec, builtin_mime_types) = if !o.nobuiltin {
    let builtin_desc_types: Vec<(Regex, Vec<String>)> =
      serde_cbor::from_slice::<Vec<(String,Vec<String>)>>(DESC_TYPES_CBOR)
      .expect("Failed to initialize: invalid built-in desc.types CBOR")
      .iter().map(|d| {
        let (r, exts) = d;
        let regex = Regex::new(&*r).expect(&*format!(
          "Failed to initialize: invalid regex in description CBOR: {}", r));
        return (regex, exts.clone());
      }).collect();

    let builtin_mime_types_vec: Vec<(String,Vec<String>)> =
      serde_cbor::from_slice::<Vec<(String,Vec<String>)>>(MIME_TYPES_CBOR)
      .expect("Failed to initialize: invalid built-in mime.types CBOR");

    let builtin_mime_types: HashMap<String,Vec<String>> = {
      let mut mt = HashMap::new();
      mt.extend(builtin_mime_types_vec.clone());
      mt
    };

    (builtin_desc_types, builtin_mime_types_vec, builtin_mime_types)
  }
  else {
    (Vec::new(), Vec::new(), HashMap::new())
  };


  let c = Cookie {
    desc: magic::Cookie::open(CookieFlags::default())
            .expect("Failed to initialize: couldn't open a magic cookie with default flags"),
    mime: magic::Cookie::open(MIME_TYPE)
            .expect("Failed to initialize: couldn't open a magic cookie with MAGIC_MIME_TYPE")
  };


  let init_mgc: MagicDatabase = match &o.magicfile {
    Some(p) => MagicDatabase::File(p),
    None    => {
      if cfg!(windows) {
        MagicDatabase::Buffer(BUILTIN_MGC)
      }
      else {
        MagicDatabase::File(DEFAULT_MGC)
      }
    }
  };

  magic_load(&c, init_mgc);


  macro_rules! message {
    ($fmt:expr, $($arg:tt)*) => {
      eprint!("{}: ", bold("fixext"));
      eprintln!($fmt, $($arg)*);
    };
  }

  macro_rules! message_path {
    ($file:expr, $fmt:expr, $($arg:tt)*) => {
      message!(concat!($fmt, " {}"), $($arg)*, $file);
    };
  }

  macro_rules! verbose_path {
    ($o:expr, $file:expr, $fmt:expr, $($arg:tt)*) => {
      if $o.verbose {
        message_path!($file, $fmt, $($arg)*);
      }
    };
  }

  macro_rules! bold_format {
    ($fmt:expr, $($arg:tt)*) => {
      bold(&*format!($fmt, $($arg)*));
    };
  }

  let desc_types: Vec<(Regex, Vec<String>)> = {
    let mut result: Vec<(Regex, Vec<String>)> = vec![];

    match matches.values_of("ovdesc") {
      None    => (),
      Some(o) => o.for_each(|d| {
        let splits: Vec<String> = d.splitn(2, '=').map(|s| s.to_string()).collect();
        let r:    &String     = &splits[0];
        let exts: Vec<String> =  splits[1].split(|c: char| ", ".contains(c))
                                          .filter(|s| !s.is_empty())
                                          .map(|s| s.to_string()).collect();

        match Regex::new(r) {
          Ok(regex) => result.push((regex, exts)),
          Err(e)    => panic!("Invalid regex in option '-Z{}': {}", r, e)
        }
      })
    };

    result.reverse();
    [&result[..], &builtin_desc_types[..]].concat()
  };


  let mime_types: HashMap<String, Vec<String>> = {
    let mut result: HashMap<String, Vec<String>> = HashMap::new();

    result.extend(builtin_mime_types);

    match matches.values_of("ovmime") {
      None    => (),
      Some(o) => o.for_each(|m| {
        let splits: Vec<String> = m.splitn(2, '=').map(|s| s.to_string()).collect();
        let m: &String = &splits[0];
        let exts: Vec<String> = splits[1].split(|c: char| ", ".contains(c))
                                         .filter(|s| !s.is_empty())
                                         .map(|s| s.to_string()).collect();

        if ! m.contains('/') {
          panic!("Invalid MIME in option '-X{}': no forward slash", m);
        }

        result.insert(m.clone(), exts);
      })
    };

    result
  };

  let types: Types = Types {
    desc: desc_types,
    mime: mime_types,
  };


  if o.dump {
    builtin_desc_types.iter().for_each(|(r, exts)| {
      println!("{}\t{}", r, exts.join(" "));
    });

    println!("__END__");

    builtin_mime_types_vec.iter().for_each(|(m, exts)| {
      println!("{} {}", m, exts.join(" "));
    });

    return;
  }


  let file_visitor: &dyn Fn(PathBuf) -> Result<(),String> = &|path| {
    let path_str = path.as_os_str().to_string_lossy().into_owned();

    if !path.exists() {
      message!("{} {}", bold("ERROR: File does not exist, skipping:"), path_str);
      return Err("file does not exist".to_string());
    }


    let (desc, mime, magic, dexts, mexts):
        (String, String, MagicMatch, Vec<String>, Vec<String>) = 'magic: {
      let desc = c.desc.file(&path).unwrap_or_default();
      let mime = c.mime.file(&path).unwrap_or_default();
      let mut dexts: Vec<String> = vec![];
      let mut mexts: Vec<String> = vec![];

      let mut result: MagicMatch = MagicMatch::None;

      if desc == mime && mime == String::default() {
        break 'magic (desc, mime, result, dexts, mexts);
      }

      for (r, exts) in &types.desc {
        if r.is_match(&*desc) {
          dexts = exts.clone();

          if *exts == vec![String::from("?")] {
            verbose_path!(o, path_str, "{}",
              bold_format!(
                "File description \"{}\" matches /{}/, extensions {:?}, is ignored:",
                desc, r, exts));
            break;
          }

          result = MagicMatch::Description(r.clone(), exts.clone());
        }
      }

      if let (Some(exts), MagicMatch::None) = (types.mime.get(&mime), &result) {
        result = MagicMatch::MIME(mime.clone(), exts.clone());
        mexts = exts.clone();
      }

      (desc.clone(), mime.clone(), result, dexts, mexts)
    };

    if o.matchinfo {
      println!("{}\0{}\0{}\0{}\0{}\0",
               path_str,
               desc, dexts.join(" "),
               mime, mexts.join(" "));
      return Ok(());
    }


    let (exts, matched_desc) = match magic {
      MagicMatch::Description(r, exts) => {
        verbose_path!(o, path_str, "{}",
          bold_format!(
            "File description \"{}\" matches /{}/, extensions {:?}:",
            desc, r, exts));
        (exts, desc.clone())
      },
      MagicMatch::MIME(m, exts) => {
        verbose_path!(o, path_str, "{}",
          bold_format!(
            "File MIME \"{}\" matches, extensions {:?}:",
            m, exts));
        (exts, mime.clone())
      },
      MagicMatch::None => {
        verbose_path!(o, path_str, "{}",
          bold_format!(
            "Unknown file type (description: \"{}\", MIME: {})",
            desc, mime));
        (vec![], String::from("(unknown)"))
      }
    };


    if o.detect {
      println!("{}: {}", path_str, matched_desc);
      return Ok(());
    }


    let (dirname, basename) = path_to_dir_base(&path);
    let dotsplits: Vec<String> = basename.clone().split('.').map(|s| s.to_string()).collect();

    let has_ext = basename.contains('.');

    let (extdot_matched, ext) = match vec_si(&dotsplits[1..], o.extdot) {
      Some(s) => (true, s.join(".")),
      None    => (false, String::from(""))
    };

    if (!extdot_matched) && has_ext {
      message_path!(path_str, "{}",
        bold_format!(
          "ERROR: the -L{} index is out of bounds for file, skipping:",
          o.extdot));
      return Err(String::from("extdot index out of bounds"));;
    }


    if exts == vec!["*"] {
      verbose_path!(o, path_str, "{}", bold("File ignored, skipping:"));
      return Ok(());
    }

    if exts.is_empty() {
      verbose_path!(o, path_str, "{}",
                    bold("No extensions matched for file, skipping:"));
      return Err(String::from("No matched extensions"));
    }

    if !ext.is_empty() && exts.contains(&ext) {
      verbose_path!(o, path_str, "{}",
                    bold("File has a valid matched extension, skipping:"));
      return Ok(());
    }


    let new_ext = String::from(&exts[0]);

    let new_basename: String =
      if o.append || !has_ext {
        basename.clone()
      }
      else {
        String::from(&basename.clone()[0..basename.len()-ext.len()-1])
      } + &*format!(".{}", new_ext);

    let new_fullname: PathBuf = {
      let mut new_fullname: PathBuf = PathBuf::from(&dirname);
      new_fullname.push(&new_basename);
      new_fullname
    };

    if new_fullname == *path {
      verbose_path!(o, path_str, "{}", bold("Suggested file name equals to old, skipping:"));
      return Err(String::from("attempted rename to same path"));
    }

    let destination_exists      = new_fullname.exists();
    let old_fullname_str_quoted = quote_filename(&path_str);
    let new_fullname_str        = new_fullname.as_os_str().to_string_lossy().into_owned();
    let new_basename_str_quoted = quote_filename(&new_basename);
    let new_fullname_str_quoted = quote_filename(&new_fullname_str);

    let do_rename: bool = if o.interactive {
      let mut rl = Editor::<()>::new();

      let prompt = format!(
        "{}: {} {} {} {}{}{} ",
        bold("fixext"),
        bold("rename"),
        old_fullname_str_quoted,
        bold("to"),
        new_basename_str_quoted,
        if destination_exists {bold(" (DESTINATION EXISTS)")} else {ANSIString::from("")},
        bold("?"));

      let readline = rl.readline(&*prompt);

      match readline {
        Ok(line) => {
          let yes: Regex = Regex::new(r"^\s*[yY]").unwrap();
          yes.is_match(&*line)
        },
        Err(ReadlineError::Interrupted) => {
          eprintln!("Received an interrupt");
          process::exit(130);
        },
        _ => false
      }
    }
    else if new_fullname.exists() && !o.force {
      message!("{} {} -> {}",
                bold("Renaming will overwrite an existing file and -f is not set, skipping:"),
                old_fullname_str_quoted,
                new_fullname_str_quoted);
      false
    }
    else {
      true
    };

    if do_rename {
      println!("{}{} -> {}",
               if o.dry { "(DRY RUN) " } else { "" },
               old_fullname_str_quoted,
               new_fullname_str_quoted);

      if o.dry {
        return Ok(());
      }

      if let Err(e) = fs::rename(path, new_fullname) {
        message_path!(path_str, "{}",
                      bold_format!("ERROR: fs::rename failed ({}):", e));
        return Err(format!("fs::rename failed: {}", e));
      };
    }


    return Ok(());
  }; // file_visitor


  let dir_visitor: &dyn Fn(PathBuf) -> Result<(),String> = &|path| {
    let path_str = path.as_os_str().to_string_lossy().into_owned();

    if o.matchinfo {
      let (desc, mime, dexts, mexts):
          (String, String, Vec<String>, Vec<String>) = 'magic: {
        let desc = c.desc.file(&path).unwrap_or_default();
        let mime = c.mime.file(&path).unwrap_or_default();

        let mut dexts: Vec<String> = vec![];
        let mut mexts: Vec<String> = vec![];

        for (r, exts) in &types.desc {
          if r.is_match(&*desc) {
            dexts = exts.clone();
            break;
          }
        }

        if let Some(exts) = types.mime.get(&mime) {
          mexts = exts.clone();
        }

        (desc, mime, dexts, mexts)
      };

      println!("{}\0{}\0{}\0{}\0{}\0",
               path_str,
               desc, dexts.join(" "),
               mime, mexts.join(" "));
    }

    if !o.recursive {
      message!("{} {}", bold("File is a directory, skipping:"), path_str);
      return Err(String::from("not recursing"));
    }

    return Ok(());
  };


  let error_visitor: &dyn Fn(PathBuf, String) = &|path, estr| {
    let path_str = path.as_os_str().to_string_lossy().into_owned();

    message_path!(path_str, "{}",
                  bold_format!("Failed to read file metadata ({}):", estr));
  };


  files.for_each(|fp| {
    visit_tree::<()>(&PathBuf::from(&fp),
                     &file_visitor, &dir_visitor, &error_visitor);
  });
}
