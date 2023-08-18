#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt, WriteBytesExt};

use emulator::*;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Editor};

#[cfg(test)]
mod tests;

mod cli;
mod emulator;
mod solver;

fn main() {
    let vm = Vm::default();

    let mut rl = DefaultEditor::new().unwrap();
    let mut cli = cli::Cli::new(vm);

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line).unwrap();
                match cli.parse_command(&line) {
                    Ok(_) => (),
                    Err(x) => println!("{:?}", x),
                }
            }
            _ => break,
        }
    }
}
