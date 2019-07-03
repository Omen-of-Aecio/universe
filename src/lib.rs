#![feature(map_get_key_value)]
#![feature(test)]
extern crate test; // Required for testing, even though extern crate is no longer needed in the 2018 version, this is a special case

#[macro_use]
extern crate serde_derive;

pub mod macros;
// ---
pub mod glocals;
pub mod mediators;

pub mod game;
