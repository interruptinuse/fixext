#![allow(dead_code)]
#![allow(clippy::needless_return)]


use std::vec::Vec;
use std::collections::HashMap;

extern crate regex;
use regex::Regex;



/*
pub fn dirname(path: PathBuf) -> PathBuf {
  return PathBuf::new();
}


pub fn basename(path: PathBuf) -> PathBuf {
  return PathBuf::new();
}
*/


pub struct Cookie {
  pub desc: magic::Cookie,
  pub mime: magic::Cookie,
}


#[derive(Debug)]
pub enum MagicMatch {
  Description(Regex, Vec<String>),
  MIME(String, Vec<String>),
  None
}


#[derive(Debug, Default)]
pub struct Opts {
  pub dry:         bool,
  pub interactive: bool,
  pub force:       bool,
  pub recursive:   bool,
  pub append:      bool,
  pub detect:      bool,
  pub dump:        bool,
  pub extdot:      i32,
  pub verbose:     bool,
}


pub struct Types {
  pub desc: Vec<(Regex, Vec<String>)>,
  pub mime: HashMap<String, Vec<String>>,
}
