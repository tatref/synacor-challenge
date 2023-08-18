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

use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Editor};

#[cfg(test)]
mod tests {
    #[test]
    fn load_program_from_file() -> Result<(), ()> {
        use super::Vm;

        let f = "challenge.bin";
        let mut vm = Vm::new();
        vm.load_program_from_file(f)
    }

    #[test]
    fn load_program_from_mem() -> Result<(), ()> {
        use super::Vm;

        let mut vm = Vm::new();
        let program = [9, 32768, 32769, 4, 19, 32768];
        vm.load_program_from_mem(&program);

        Ok(())
    }
}

mod game {
    use regex::Regex;

    struct GameSolver {
        game: Game,
        //levels: GraphMap<Game, ()>;
    }

    #[derive(Copy, Clone, Debug)]
    enum GameState {
        Dead,
        Playing,
    }
    #[derive(Clone, Debug)]
    struct Game {
        state: GameState,
        level: Level,
    }

    #[derive(Clone, Debug)]
    struct Level {
        name: String,
        things: Vec<String>,
        exits: Vec<String>,
    }

    impl Level {
        fn from(raw: &str) -> Option<Self> {
            let re_name = Regex::new(r"^== (\w+) ==\n").unwrap();
            let name = re_name
                .captures(raw)
                .expect("No level name")
                .get(1)
                .unwrap()
                .as_str()
                .into();

            fn get_things(raw: &str) -> Vec<String> {
                let re_things = Regex::new(r"(?s)There are \d+ things:\n([^\n]+\n)+").unwrap();
                let things_str = match re_things.captures(raw) {
                    Some(x) => x.get(0).unwrap().as_str(),
                    None => return Vec::new(),
                };

                let things = things_str
                    .lines()
                    .skip(1)
                    .map(|line| line.get(2..).unwrap().to_string())
                    .collect::<Vec<_>>();
                things
            }
            let things = get_things(raw);

            fn get_exits(raw: &str) -> Vec<String> {
                let re_exits = Regex::new(r"(?s)There are \d+ exits:\n([^\n]+\n)+").unwrap();
                let exits_str = match re_exits.captures(raw) {
                    Some(x) => x.get(0).unwrap().as_str(),
                    None => return Vec::new(),
                };

                let exits = exits_str
                    .lines()
                    .skip(1)
                    .map(|line| line.get(2..).unwrap().to_string())
                    .collect::<Vec<_>>();
                exits
            }
            let exits = get_exits(raw);

            let level = Level {
                name,
                things,
                exits,
            };

            Some(level)
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Value {
    Number(u16),
    Register(usize),
    Invalid,
}
impl Value {
    fn new(v: u16) -> Self {
        match v {
            0..=32767 => Value::Number(v),
            32768..=32775 => Value::Register((v - 32768) as usize),
            32776..=65535 => Value::Invalid,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum Opcode {
    /* 0  */ Halt,
    /* 1  */ Set(Value, Value),
    /* 2  */ Push(Value),
    /* 3  */ Pop(Value),
    /* 4  */ Eq(Value, Value, Value),
    /* 5  */ Gt(Value, Value, Value),
    /* 6  */ Jmp(Value, Value),
    /* 7  */ Jt(Value, Value),
    /* 8  */ Jf(Value, Value),
    /* 9  */ Add(Value, Value, Value),
    /* 10 */ Mult(Value, Value, Value),
    /* 11 */ Mod(Value, Value, Value),
    /* 12 */ And(Value, Value, Value),
    /* 13 */ Or(Value, Value, Value),
    /* 14 */ Not(Value, Value),
    /* 15 */ Rmem(Value, Value),
    /* 16 */ Wmem(Value, Value),
    /* 17 */ Call(Value),
    /* 18 */ Ret,
    /* 19 */ Out(Value),
    /* 20 */ In(Value),
    /* 21 */ Noop,
}

const MEM_SIZE: usize = 32768;

#[derive(Clone)]
pub struct Vm {
    memory: [u16; MEM_SIZE],
    registers: [u16; 8],
    stack: Vec<u16>,
    /// Instruction Pointer (next instruction)
    ip: usize,
    /// Program Counter
    pc: usize,

    state: VmState,

    output_buffer: Vec<char>,
    input_buffer: VecDeque<char>,

    out_messages: Vec<String>,
}

impl PartialEq for Vm {
    fn eq(&self, other: &Self) -> bool {
        for (x, y) in self.memory.iter().zip(other.memory.iter()) {
            if x != y {
                return false;
            }
        }
        if self.registers != other.registers {
            return false;
        }
        if self.ip != other.ip {
            return false;
        }
        if self.pc != other.pc {
            return false;
        }
        if self.output_buffer != other.output_buffer {
            return false;
        }
        if self.input_buffer != other.input_buffer {
            return false;
        }

        true
    }
}

impl fmt::Debug for Vm {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "VM {{")?;
        writeln!(f, "  registers: {:?}", self.registers)?;
        writeln!(f, "  stack: {:?}", self.stack)?;
        writeln!(f, "  ip: {:?}", self.ip)?;
        writeln!(f, "  pc: {:?}", self.pc)?;
        writeln!(f, "  state: {:?}", self.state)?;
        writeln!(f, "  memory: [...]")?;
        write!(f, "}}")
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
enum VmState {
    Running,
    Halted,
    WaitingForInput,
}

impl Vm {
    fn new() -> Self {
        Vm {
            memory: [0u16; MEM_SIZE],
            registers: [0u16; 8],
            stack: Vec::new(),
            ip: 0,
            pc: 0,

            state: VmState::Running,

            output_buffer: Vec::new(),
            input_buffer: VecDeque::new(),

            out_messages: Vec::new(),
        }
    }

    fn default() -> Self {
        let mut vm = Vm::new();
        vm.load_program_from_file("challenge.bin")
            .expect("Unable to load default 'challenge.bin'");

        vm
    }

    fn load_program_from_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), ()> {
        let mut f = File::open(path).map_err(|_| ())?;
        let mut buff = Vec::new();

        f.read_to_end(&mut buff).map_err(|_| ())?;

        let data: Vec<_> = buff
            .chunks(2)
            .map(|x| {
                //let pair = x.to_vec();
                LittleEndian::read_u16(x)
            })
            .collect();

        if data.len() > MEM_SIZE {
            panic!("File is too big");
        }
        self.memory[..data.len()].copy_from_slice(&data[..]);

        Ok(())
    }

    fn load_program_from_mem(&mut self, program: &[u16]) {
        self.memory[..program.len()].copy_from_slice(program);
    }

    fn run(&mut self) {
        self.state = VmState::Running;
        let starting_pc = self.pc;

        while self.state == VmState::Running {
            self.step();
        }
        let elapsed = self.pc - starting_pc;

        println!(
            "\nStopped after {} instructions. State is now {:?}",
            elapsed, self.state
        );

        if self.state == VmState::Halted {
            let message = self.output_buffer.iter().collect::<String>();
            self.out_messages.push(message.clone());

            println!("\n\nHalted. Messages:");
            for message in &self.out_messages {
                println!("{}", message);
            }
            println!("\n\nHalted");
        }
    }

    fn feed(&mut self, line: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.state != VmState::WaitingForInput {
            return Err(format!("State is {:?}, can't feed", self.state).into());
        }
        if !self.input_buffer.is_empty() {
            return Err("Trying to feed but buffer is not empty".into());
        }

        self.input_buffer = line.chars().collect();
        self.input_buffer.push_back('\n');
        //self.state = VmState::Running;

        Ok(())
    }

    fn step(&mut self) {
        let (instruction, size) = self.fetch();

        let next_instruction_ptr = self.ip + size;
        self.execute(&instruction, next_instruction_ptr);
        self.pc += 1;
    }

    fn fetch(&self) -> (Opcode, usize) {
        let instr_type = self.memory[self.ip];

        match instr_type {
            0 => (Opcode::Halt, 1),
            1 => (
                Opcode::Set(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            2 => (Opcode::Push(Value::new(self.memory[self.ip + 1])), 2),
            3 => (Opcode::Pop(Value::new(self.memory[self.ip + 1])), 2),
            4 => (
                Opcode::Eq(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            5 => (
                Opcode::Gt(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            6 => (
                Opcode::Jmp(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            7 => (
                Opcode::Jt(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            8 => (
                Opcode::Jf(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            9 => (
                Opcode::Add(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            10 => (
                Opcode::Mult(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            11 => (
                Opcode::Mod(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            12 => (
                Opcode::And(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            13 => (
                Opcode::Or(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                    Value::new(self.memory[self.ip + 3]),
                ),
                4,
            ),
            14 => (
                Opcode::Not(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            15 => (
                Opcode::Rmem(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            16 => (
                Opcode::Wmem(
                    Value::new(self.memory[self.ip + 1]),
                    Value::new(self.memory[self.ip + 2]),
                ),
                3,
            ),
            17 => (Opcode::Call(Value::new(self.memory[self.ip + 1])), 2),
            18 => (Opcode::Ret, 1),
            19 => (Opcode::Out(Value::new(self.memory[self.ip + 1])), 2),
            20 => (Opcode::In(Value::new(self.memory[self.ip + 1])), 2),
            21 => (Opcode::Noop, 1),
            x => unreachable!("Fetch: unknown instr '{}'", x),
        }
    }

    fn execute(&mut self, instruction: &Opcode, next_instruction_ptr: usize) {
        //println!("{:?}", instruction);

        self.ip = next_instruction_ptr;

        match instruction {
            Opcode::Halt => self.state = VmState::Halted,
            Opcode::Set(a, b) => {
                let val = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val;
            }
            Opcode::Push(a) => {
                let val = self.get_value(a).expect("Invalid number");

                self.stack.push(val);
            }
            Opcode::Pop(a) => {
                let val = self.stack.pop().expect("Pop: empty stack");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val;
            }
            Opcode::Eq(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");

                let val_a = if val_b == val_c { 1 } else { 0 };

                let reg = self.get_register(a).expect("Not a register");
                self.registers[reg] = val_a;
            }
            Opcode::Gt(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");

                let val_a = if val_b > val_c { 1 } else { 0 };

                let reg = self.get_register(a).expect("Not a register");
                self.registers[reg] = val_a;
            }
            Opcode::Jmp(a, b) => {
                self.ip = self.get_value(a).expect("Invalid number") as usize;
            }
            Opcode::Jt(a, b) => {
                let must_jump = self.get_value(a).expect("Invalid number") != 0;

                if must_jump {
                    self.ip = self.get_value(b).expect("Invalid number") as usize;
                }
            }
            Opcode::Jf(a, b) => {
                let must_jump = self.get_value(a).expect("Invalid number") == 0;

                if must_jump {
                    self.ip = self.get_value(b).expect("Invalid number") as usize;
                }
            }
            Opcode::Add(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b + val_c) % 32768; //TODO wrapping_add?
            }
            Opcode::Mult(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val_b.wrapping_mul(val_c) % 32768;
            }
            Opcode::Mod(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = val_b % val_c;
            }
            Opcode::And(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b & val_c) % 32768;
            }
            Opcode::Or(a, b, c) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let val_c = self.get_value(c).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (val_b | val_c) % 32768;
            }
            Opcode::Not(a, b) => {
                let val_b = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                self.registers[reg] = (!val_b) % 32768;
            }
            Opcode::Rmem(a, b) => {
                let addr = self.get_value(b).expect("Invalid number");
                let reg = self.get_register(a).expect("Not a register");

                let val = self.memory[addr as usize];

                self.registers[reg] = val;
            }
            Opcode::Wmem(a, b) => {
                let val = self.get_value(b).expect("Invalid number");
                let addr = self.get_value(a).expect("Not a register");

                self.memory[addr as usize] = val;
            }
            Opcode::Call(a) => {
                let addr = self.get_value(a).expect("Invalid number");

                self.stack.push(self.ip as u16);
                self.ip = addr as usize;
            }
            Opcode::Ret => match self.stack.pop() {
                Some(addr) => self.ip = addr as usize,
                None => self.state = VmState::Halted,
            },
            Opcode::Out(a) => {
                let c = self.get_value(a).expect("Invalid number");

                self.output_buffer.push(c as u8 as char);
                //print!("{}", c as u8 as char);
            }
            Opcode::In(a) => {
                let reg = self.get_register(a).expect("In: not a register");

                match self.input_buffer.pop_front() {
                    Some(c) => {
                        // just feed the current input
                        self.registers[reg] = c as u16;
                    }
                    None => {
                        // asking for new input
                        // first, flush current output
                        let out = self.output_buffer.iter().collect::<String>(); //TODO: separate function
                        self.out_messages.push(out.clone());
                        print!("{}", out);
                        self.output_buffer = Vec::new();

                        self.state = VmState::WaitingForInput;
                        self.ip -= 2; // size of `In` instruction
                    }
                }
            }
            Opcode::Noop => (),
        }
    }

    fn get_value(&self, value: &Value) -> Option<u16> {
        match value {
            Value::Number(x) => Some(*x),
            Value::Register(x) => Some(self.registers[*x]),
            Value::Invalid => None,
        }
    }

    fn get_register(&self, value: &Value) -> Option<usize> {
        match value {
            Value::Number(x) => None,
            Value::Register(x) => Some(*x),
            Value::Invalid => None,
        }
    }
}

mod cli {
    use crate::VmState;

    use super::Vm;
    //use clap::{App, AppSettings, Arg, SubCommand};
    use clap::{builder::RangedU64ValueParser, Arg, Command};

    #[derive(Debug)]
    pub struct Snapshot {
        name: String,
        vm: Vm,
    }

    pub struct Cli {
        pub cli: Command,

        pub vm: Vm,
        pub snapshots: Vec<Snapshot>,
    }

    impl Cli {
        pub fn new(vm: Vm) -> Self {
            let cli = Command::new("cli")
                .subcommand_required(true)
                .no_binary_name(true)
                .subcommand(Command::new("helpme"))
                .subcommand(Command::new("vm"))
                .subcommand(Command::new("run").alias("r"))
                .subcommand(Command::new("input").alias("i").arg(Arg::new("line")))
                .subcommand(
                    Command::new("snapshot")
                        .alias("snap")
                        .subcommand(
                            Command::new("take")
                                .alias("t")
                                .arg(Arg::new("name").required(true)),
                        )
                        .subcommand(
                            Command::new("revert")
                                .alias("r")
                                .arg(Arg::new("idx").required(true)),
                        )
                        .subcommand(Command::new("list").alias("l")),
                )
                .subcommand(
                    Command::new("step").alias("s").arg(
                        Arg::new("count")
                            .value_parser(RangedU64ValueParser::<u32>::new())
                            .default_value("1"),
                    ),
                );

            Self {
                cli,
                vm,
                snapshots: Vec::new(),
            }
        }

        fn show_help(&self) -> Result<(), ()> {
            print!("HELP!");

            Ok(())
        }

        fn take_snapshot(&mut self, name: &str) {
            self.snapshots.push(Snapshot {
                name: name.to_string(),
                vm: self.vm.clone(),
            });
        }

        fn revert_snapshot(&mut self, idx: usize) {
            if idx < self.snapshots.len() {
                let snap = self.snapshots.remove(idx);
                self.vm = snap.vm;
                println!("Reverted {}", idx);
            } else {
                println!("Can revert snapshot {}", idx);
            }
        }

        pub fn parse_command(
            &mut self,
            input_line: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            if input_line.split_whitespace().next().is_none() {
                // empy command
                return Ok(());
            }

            let argv = input_line.split_whitespace();
            let args = match self.cli.clone().try_get_matches_from(argv.clone()) {
                Ok(args) => args,
                Err(_) => match self.vm.feed(input_line) {
                    Ok(_) => {
                        self.vm.run();
                        return Ok(());
                    }
                    Err(e) => {
                        println!("Invalid command, tried feeding, but didn't work either");
                        return Err(e);
                    }
                },
            };

            match args.subcommand() {
                Some(("run", sub)) => self.vm.run(),
                Some(("input", sub)) => self.vm.feed(sub.get_one::<String>("line").unwrap())?,
                Some(("vm", sub)) => {
                    println!("{:?}", self.vm);
                }
                Some(("snapshot", sub)) => match sub.subcommand() {
                    Some(("take", subsub)) => {
                        let name = subsub.get_one::<String>("name").unwrap();
                        self.take_snapshot(name);
                    }
                    Some(("revert", subsub)) => {
                        let idx = *subsub.get_one("idx").unwrap();
                        self.revert_snapshot(idx);
                    }
                    Some(("list", _)) => {
                        println!("{} snapshots:", self.snapshots.len());
                        println!("{:?}", self.snapshots);
                    }
                    _ => {
                        let name = format!("{:03}", self.snapshots.len());
                        self.take_snapshot(&name);
                    }
                },
                Some(("step", sub)) => {
                    let count: u32 = *sub.get_one("count").unwrap();
                    for i in 0..count {
                        self.vm.step();
                    }
                }
                Some(("helpme", _)) => {
                    self.cli.print_long_help().unwrap();
                }
                Some((x, y)) => unimplemented!("Unknown command {x:?}"),
                None => (),
            }

            Ok(())
        } // end fn parse_command
    }
}

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
