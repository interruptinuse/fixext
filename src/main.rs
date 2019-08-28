#![allow(clippy::needless_return)]
#![allow(clippy::cognitive_complexity)]


const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const AUTHORS: Option<&'static str> = option_env!("CARGO_PKG_AUTHORS");
const DESCRIP: Option<&'static str> = option_env!("CARGO_PKG_DESCRIPTION");


// TODO: build.rs, mime.types.txt, cargo env variable (see above consts)
const MIME_TYPES_CBOR: &[u8] = include_bytes!("mime.types.cbor");
const DESC_TYPES_CBOR: &[u8] = include_bytes!("desc.types.cbor");


use std::fs;
use std::process;
use std::convert::TryInto;
use std::str::FromStr;
use std::path;
use std::path::PathBuf;
use std::vec::Vec;
use std::clone::Clone;
use std::collections::HashMap;


extern crate magic;
use magic::CookieFlags;
use magic::flags::MIME_TYPE;

//extern crate tree_magic;

extern crate serde;
extern crate serde_cbor;

extern crate glib;
use glib::{path_get_dirname, path_get_basename};

extern crate regex;
use regex::Regex;

extern crate ansi_term;
use ansi_term::Style;
use ansi_term::ANSIString;

#[macro_use]
extern crate clap;

extern crate rustyline;
use rustyline::error::ReadlineError;
use rustyline::Editor;

extern crate shellwords;

//extern crate fixext;
use fixext::Cookie;
use fixext::MagicMatch;
use fixext::Opts;
use fixext::Types;



fn bold(s: &str) -> ANSIString<> {
  return Style::default().bold().paint(s);
}


fn path_to_dir_base(s: &path::Path) -> (String, String) {
  macro_rules! __ {
    ($($fn:ident),*) => {
      ($(
        {
          let component: String;

          if let Some(pbuf) = $fn(s) {
            if let Some(os_str) = pbuf.to_str() {
              component = String::from_str(os_str).unwrap();
            }
            else { panic!("non-Unicode characters in string") };
          }
          else { panic!("glib::{} failed", stringify!($fn)) };

          component
        }
      ),*)
    };
  }

  let (d, b) = __!(path_get_dirname, path_get_basename);
  return (if d == "." {String::from("")} else {d}, b);
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


fn match_file(
    fpath:  &str,
    cookie: &Cookie,
    descs:  &[(Regex,Vec<String>)],
    mimes:  &HashMap<String,Vec<String>>
) -> (String, String, MagicMatch) {

  let desc = cookie.desc.file(&fpath).unwrap();
  let mime = cookie.mime.file(&fpath).unwrap();

  for (r, exts) in descs {
    if r.is_match(&*desc) {
      return (desc.clone(), mime.clone(), MagicMatch::Description(r.clone(), exts.clone()));
    }
  }

  if let Some(exts) = mimes.get(&mime) {
    return (desc.clone(), mime.clone(), MagicMatch::MIME(mime, exts.clone()));
  }

  return (desc.clone(), mime.clone(), MagicMatch::None);
}



fn main() {
  let app = clap_app!(fixext =>
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
    (@arg detect:      -F ... "Only print detected types (like `file --mime-type`)")
    (@arg dump:        -D ... "Print known descriptions/MIME types and associated extensions")
    (@arg extdot:      -L [IDX] ...           number_of_values(1)
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


  let c = Cookie {
    desc: magic::Cookie::open(CookieFlags::default())
            .expect("Failed to initialize: couldn't open a magic cookie with default flags"),
    mime: magic::Cookie::open(MIME_TYPE)
            .expect("Failed to initialize: couldn't open a magic cookie with MAGIC_MIME_TYPE")
  };

  c.desc.load(&["/usr/share/misc/magic.mgc"])
    .expect("Failed to initialize: could not load desc.types magic");
  c.mime.load(&["/usr/share/misc/magic.mgc"])
    .expect("Failed to initialize: could not load mime.types magic");

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
    get_flag!(detect);
    get_flag!(dump);
    get_flag!(verbose);

    o.extdot = match matches.values_of("extdot") {
      Some(v)  => v.last().unwrap().parse::<i32>().unwrap(),
      None     => -1
    };

    o
  };

  macro_rules! message {
    ($($arg:tt)*) => {
      eprint!("{}: ", bold("fixext"));
      eprintln!($($arg)*);
    };
  }

  macro_rules! verbose {
    ($o:expr, $($arg:tt)*) => {
      if $o.verbose {
        message!($($arg)*);
      }
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


  fn process_file(
      fp:     &PathBuf,
      opts:   &Opts,
      cookie: &Cookie,
      types:  &Types) {
    // XXX: rest_path must be relative, or it will overwrite prefix_path
    // XXX: however, it works with dirname/basename
    let full_path_str = fp.as_os_str().to_string_lossy().into_owned();

    let md = match fs::metadata(&fp) {
      Ok(m)  => m,
      Err(e) => {
        message!("{} {}",
                 bold(&*format!("ERROR: Cannot read file metadata ({}):", e)),
                 full_path_str);
        return;
      }
    };

    //verbose!(opts, "full_path_str: {}", full_path_str);

    if !fp.exists() {
      message!("{} {}", bold("ERROR: File does not exist, skipping:"), full_path_str);
      return;
    }

    if md.is_dir() {
      if !opts.recursive {
        message!("{} {}", bold("File is a directory, skipping:"), full_path_str);
        return;
      }

      match std::fs::read_dir(&fp) {
        Ok(rd) => {
          for entry in rd {
            match entry {
              Ok(d)  => {
                // TODO: replace file_name() with path() and get rid of prefix
                process_file(&d.path(), opts, cookie, types);
              }
              Err(e) => {
                message!("{} {}",
                         bold(&*format!("ERROR: Invalid entry in directory ({}):", e)),
                         full_path_str);
              }
            }
          }
        },
        Err(e) => {
          message!("{} {}",
                   bold(&*format!("ERROR: fs::read_dir error ({}), skipping:", e)),
                   full_path_str);
          return;
        }
      }

      return;
    };

    let (desc, mime, magic) = match_file(&full_path_str, &cookie, &types.desc, &types.mime);

    //// XXX: remove
    //verbose!(opts, "file={} desc={} mime={} match={:?}", full_path_str, desc, mime, magic);

    let (exts, matched_desc) = match magic {
      MagicMatch::Description(r, exts) => {
        verbose!(opts, "{} {}",
                 bold(&*format!("File description \"{}\" matches /{}/, extensions {:?}:",
                                desc, r, exts)), full_path_str);
        (exts, desc.clone())
      },
      MagicMatch::MIME(m, exts) => {
        verbose!(opts, "{} {}",
                 bold(&*format!("File MIME \"{}\" matches, extensions {:?}:",
                                m, exts)), full_path_str);
        (exts, mime.clone())
      },
      MagicMatch::None => {
        verbose!(opts, "{} {}",
                 bold(&*format!("Unknown file type (description: \"{}\", MIME: {})",
                                desc, mime)), full_path_str);
        (vec![], String::from("(unknown)"))
      }
    };

    if opts.detect {
      println!("{}: {}", full_path_str, matched_desc);
      return;
    }

    // TODO: bikeshed dirname/basename
    let (dirname, basename) = path_to_dir_base(&fp);
    let dotsplits: Vec<String> = basename.clone().split('.').map(|s| s.to_string()).collect();

    let has_ext = basename.contains('.');

    let (extdot_matched, ext) = match vec_si(&dotsplits[1..], opts.extdot) {
      Some(s) => (true, s.join(".")),
      None    => (false, String::from(""))
    };

    //// XXX: remove
    //verbose!(opts, "ext match: {:?}", (&extdot_matched, &ext));

    if (!extdot_matched) && has_ext {
      message!("{} {}",
               bold(&*format!("ERROR: the -L{} index does not match an extension, skipping:", opts.extdot)), full_path_str);
      return;
    }

    //// XXX: remove
    //verbose!(opts, "{:?}", (&dirname, &basename));
    //verbose!(opts, "dotsplits: {:?}", dotsplits);
    //verbose!(opts, "ext: {:?}", ext);

    if exts == vec!["*"] {
      verbose!(opts, "{} {}", bold("File ignored, skipping:"), full_path_str);
      return;
    }

    if exts.is_empty() {
      verbose!(opts, "{} {}", bold("No extensions matched for file, skipping"), full_path_str);
      return;
    }

    if !ext.is_empty() && exts.contains(&ext) {
      return;
    }

    let new_ext = String::from(&exts[0]);

    let new_basename: String =
      if opts.append || !has_ext {
        basename.clone()
      }
      else {
        String::from(&basename.clone()[0..basename.len()-ext.len()-1])
      } + &*format!(".{}", new_ext);

    //verbose!(opts, "new_basename: {}", new_basename);

    let new_fullname: PathBuf = {
      let mut new_fullname: PathBuf = PathBuf::from(&dirname);
      new_fullname.push(new_basename);
      new_fullname
    };

    //verbose!(opts, "new_fullname: {:?}", &new_fullname);

    if new_fullname == *fp {
      return;
    }

    let destination_exists = new_fullname.exists();
    let old_fullname_str_quoted = shellwords::escape(&full_path_str);
    let new_fullname_str = new_fullname.as_os_str().to_string_lossy().into_owned();
    let new_fullname_str_quoted = shellwords::escape(&new_fullname_str);

    let do_rename: bool = if opts.interactive {
      let mut rl = Editor::<()>::new();

      let prompt = format!(
        "{}: {} '{}' {} '{}'{}{} ",
        bold("fixext"),
        bold("rename"),
        old_fullname_str_quoted,
        bold("to"),
        new_fullname_str_quoted,
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
    else {
      true
    };

    if do_rename {
      println!("{} -> {}", old_fullname_str_quoted, new_fullname_str_quoted);

      if let Err(e) = fs::rename(fp, new_fullname) {
        message!("{} {}", bold(&*format!("ERROR: fs::rename failed ({}):", e)), full_path_str);
      };
    }
  };


  files.for_each( |fp| {
    process_file(&PathBuf::from(&fp), &o, &c, &types);
  });
}
