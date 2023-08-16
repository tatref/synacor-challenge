#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

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
        use super::VM;

        let f = "challenge.bin";
        let mut vm = VM::new();
        vm.load_program_from_file(f)
    }

    #[test]
    fn load_program_from_mem() -> Result<(), ()> {
        use super::VM;

        let mut vm = VM::new();
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
            _ => unreachable!(),
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
pub struct VM {
    memory: [u16; MEM_SIZE],
    registers: [u16; 8],
    stack: Vec<u16>,
    /// Instruction Pointer (next instruction)
    ip: usize,
    /// Program Counter
    pc: usize,

    output: Vec<char>,
    input: Vec<char>,
}

impl PartialEq for VM {
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
        if self.output != other.output {
            return false;
        }
        if self.input != other.input {
            return false;
        }

        true
    }
}

impl fmt::Debug for VM {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "VM {{")?;
        writeln!(f, "  registers: {:?}", self.registers)?;
        writeln!(f, "  stack: {:?}", self.stack)?;
        writeln!(f, "  ip: {:?}", self.ip)?;
        writeln!(f, "  pc: {:?}", self.pc)?;
        writeln!(f, "  memory: [...]")?;
        write!(f, "}}")
    }
}

impl VM {
    fn new() -> Self {
        VM {
            memory: [0u16; MEM_SIZE],
            registers: [0u16; 8],
            stack: Vec::new(),
            ip: 0,
            pc: 0,

            output: Vec::new(),
            input: Vec::new(),
        }
    }

    fn default() -> Self {
        let mut vm = VM::new();
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
                let pair = x.iter().map(|x| *x).collect::<Vec<u8>>();
                LittleEndian::read_u16(&pair)
            })
            .collect();

        if data.len() >= MEM_SIZE {
            panic!("File is too big");
        }
        for i in 0..data.len() {
            self.memory[i] = data[i];
        }

        Ok(())
    }

    fn load_program_from_mem(&mut self, program: &[u16]) {
        for i in 0..program.len() {
            self.memory[i] = program[i];
        }
    }

    fn run_until_halt(&mut self) {
        while !self.step() {}

        print!("\n\nHalt: {}", self.output.iter().collect::<String>());
    }

    fn step(&mut self) -> bool {
        let (instruction, size) = self.fetch();

        let next_instruction_ptr = self.ip + size;
        let must_exit = self.execute(&instruction, next_instruction_ptr);
        self.pc += 1;

        must_exit
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

    fn execute(&mut self, instruction: &Opcode, next_instruction_ptr: usize) -> bool {
        //println!("{:?}", instruction);

        self.ip = next_instruction_ptr;

        let mut must_halt = false;
        match instruction {
            Opcode::Halt => must_halt = true,
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
                None => must_halt = true,
            },
            Opcode::Out(a) => {
                let c = self.get_value(a).expect("Invalid number");

                self.output.push(c as u8 as char);
                //print!("{}", c as u8 as char);
            }
            Opcode::In(a) => {
                let reg = self.get_register(a).expect("In: not a register");

                match self.input.pop() {
                    Some(c) => {
                        // just feed the current input
                        self.registers[reg as usize] = c as u16;
                    }
                    None => {
                        // asking for new output
                        // first, flush current output
                        let out = self.output.iter().collect::<String>(); //TODO: separate function
                        print!("{}", out);
                        self.output = Vec::new();

                        // read input
                        let mut buff = String::new();
                        io::stdin()
                            .read_line(&mut buff)
                            .expect("In: unable to read");
                        self.input = buff.chars().collect();
                        self.input.reverse();

                        // then feed 1 char
                        let c = self.input.pop().unwrap();
                        self.registers[reg as usize] = c as u16;
                    }
                }
            }
            Opcode::Noop => (),
        }

        must_halt
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
    use super::VM;
    use clap::{App, AppSettings, Arg, SubCommand};

    pub struct Cli<'a, 'b> {
        pub app: App<'a, 'b>,

        pub vm: VM,
        pub snapshots: Vec<VM>,
    }

    impl<'a, 'b> Cli<'a, 'b> {
        pub fn new(vm: VM) -> Self {
            let app = App::new("cli")
                .setting(AppSettings::NoBinaryName)
                .subcommand(SubCommand::with_name("help").alias("h"))
                .subcommand(SubCommand::with_name("vm"))
                .subcommand(
                    SubCommand::with_name("snapshot")
                        .alias("snap")
                        .subcommand(SubCommand::with_name("take").alias("t"))
                        .subcommand(
                            SubCommand::with_name("revert")
                                .alias("r")
                                .arg(Arg::with_name("idx").required(true)),
                        )
                        .subcommand(SubCommand::with_name("list").alias("l")),
                )
                .subcommand(
                    SubCommand::with_name("step")
                        .alias("s")
                        .arg(Arg::with_name("count").default_value("1")),
                );

            Cli {
                app,
                vm,
                snapshots: Vec::new(),
            }
        }

        fn show_help(&self) -> Result<(), ()> {
            print!("HELP!");

            Ok(())
        }

        fn take_snapshot(&mut self) {
            self.snapshots.push(self.vm.clone());
        }

        fn revert_snapshot(&mut self, idx: usize) {
            if idx < self.snapshots.len() {
                let snap = self.snapshots.remove(idx);
                self.vm = snap;
                println!("Reverted {}", idx);
            } else {
                println!("Can revert snapshot {}", idx);
            }
        }

        pub fn parse_command(&mut self, raw: &str) -> Result<(), ()> {
            if raw.split_whitespace().next().is_none() {
                // empy command
                return Ok(());
            }

            let argv = raw.split_whitespace();

            let args = self.app.clone().get_matches_from_safe(argv).map_err(|e| {
                println!("Unknown command");
                ()
            })?;
            //println!("{:#?}", args);

            match args.subcommand() {
                ("vm", Some(sub)) => {
                    println!("{:?}", self.vm);
                }
                ("snapshot", Some(sub)) => match sub.subcommand() {
                    ("take", _) => self.take_snapshot(),
                    ("revert", Some(subsub)) => {
                        self.revert_snapshot(subsub.value_of("idx").unwrap().parse().unwrap())
                    }
                    ("list", _) => {
                        println!("{} snapshots:", self.snapshots.len());
                        println!("{:?}", self.snapshots);
                    }
                    _ => self.take_snapshot(),
                },
                ("step", Some(sub)) => {
                    let count: usize = sub.value_of("count").unwrap().parse().unwrap();
                    for i in 0..count {
                        self.vm.step();
                    }
                }
                ("help", _) => {
                    self.app.print_long_help();
                }
                _ => unreachable!(),
            }

            Ok(())
        } // end fn parse_command
    }
}

fn main() {
    let vm = VM::default();

    let mut rl = DefaultEditor::new().unwrap();
    let mut cli = cli::Cli::new(vm);

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
                let _ = cli.parse_command(&line);
            }
            _ => break,
        }
    }
}
