use serde::{Deserialize, Serialize};
use std::fmt;

use crate::emulator::Vm;

#[derive(Copy, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Val {
    Num(u16),
    Reg(usize),
    Invalid,
}

impl std::str::FromStr for Val {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.chars().all(|c| c.is_numeric()) {
            Ok(Val::Num(s.parse()?))
        } else {
            let l_par = s.find('(');
            let size = s.chars().count();
            let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];

            let reg = inner.parse()?;

            Ok(Val::Reg(reg))
        }
    }
}

impl Val {
    pub fn new(v: u16) -> Self {
        match v {
            0..=32767 => Val::Num(v),
            32768..=32775 => Val::Reg((v - 32768) as usize),
            32776..=65535 => Val::Invalid,
        }
    }

    pub fn as_binary(&self) -> u16 {
        match self {
            Val::Num(v) => *v,
            Val::Reg(r) => *r as u16 + 32768,
            Val::Invalid => 32776,
        }
    }
}
impl fmt::Debug for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Num(arg0) => write!(f, "{}", arg0),
            Self::Reg(arg0) => f.debug_tuple("Reg").field(arg0).finish(),
            Self::Invalid => write!(f, "Invalid"),
        }
    }
}

#[repr(u32)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum Opcode {
    Halt = 1 << 0,
    Set(Val, Val) = 1 << 1,
    Push(Val) = 1 << 2,
    Pop(Val) = 1 << 3,
    Eq(Val, Val, Val) = 1 << 4,
    Gt(Val, Val, Val) = 1 << 5,
    /// jump to `a`
    Jmp(Val) = 1 << 6,
    /// if `a` is nonzero, jump to `b`
    Jt(Val, Val) = 1 << 7,
    /// if `a`  is zero, jump to `b`
    Jf(Val, Val) = 1 << 8,
    Add(Val, Val, Val) = 1 << 9,
    Mult(Val, Val, Val) = 1 << 10,
    Mod(Val, Val, Val) = 1 << 11,
    And(Val, Val, Val) = 1 << 12,
    Or(Val, Val, Val) = 1 << 13,
    Not(Val, Val) = 1 << 14,
    Rmem(Val, Val) = 1 << 15,
    Wmem(Val, Val) = 1 << 16,
    /// write the address of the next instruction to the stack and jump to `a`
    Call(Val) = 1 << 17,
    /// remove the top element from the stack and jump to it; empty stack = halt
    Ret = 1 << 18,
    Out(Val) = 1 << 19,
    In(Val) = 1 << 20,
    Noop = 1 << 21,
    __Invalid = 1 << 31,
}

impl std::str::FromStr for Opcode {
    type Err = Box<dyn std::error::Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Opcode::*;

        let l_par = s.find('(');
        let size = s.chars().count();
        let opcode = match s.to_lowercase().split('(').next().unwrap() {
            "halt" => Halt,
            "set" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Set(a, b)
            }
            "push" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Push(a)
            }
            "pop" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Pop(a)
            }
            "eq" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Eq(a, b, c)
            }
            "gt" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Gt(a, b, c)
            }
            "jmp" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Jmp(a)
            }
            "jt" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Jt(a, b)
            }
            "jf" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Jf(a, b)
            }
            "add" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Add(a, b, c)
            }
            "mult" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Mult(a, b, c)
            }
            "mod" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Mod(a, b, c)
            }
            "and" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                And(a, b, c)
            }
            "or" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let c = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Or(a, b, c)
            }
            "not" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Not(a, b)
            }
            "rmem" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Rmem(a, b)
            }
            "wmem" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                let b = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Wmem(a, b)
            }
            "call" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Call(a)
            }
            "ret" => Opcode::Ret,
            "out" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                Out(a)
            }
            "in" => {
                let inner = &s[1 + l_par.ok_or("Missing left par")?..(size - 1)];
                let mut split = inner.split(',');
                let a = split
                    .next()
                    .ok_or("missing first operand")?
                    .trim()
                    .parse()?;
                In(a)
            }
            "noop" => Opcode::Noop,
            _ => return Err("Unknown opcode".into()),
        };

        Ok(opcode)
    }
}

impl Opcode {
    pub fn discriminant(&self) -> u32 {
        unsafe { *(self as *const Self as *const u32) }
    }

    pub fn resolve_opcode(&self, vm: &Vm) -> Option<Opcode> {
        use Opcode::*;
        use Val::*;

        match self {
            Call(Num(_)) => None,
            Call(Reg(reg)) => Some(Call(Num(vm.get_registers()[*reg] as u16))),
            Call(Invalid) => unimplemented!(),
            _ => unimplemented!(),
        }
    }

    pub fn size(&self) -> usize {
        use Opcode::*;

        match self {
            Halt => 1,
            Set(_, _) => 3,
            Push(_) => 2,
            Pop(_) => 2,
            Eq(_, _, _) => 4,
            Gt(_, _, _) => 4,
            Jmp(_) => 2,
            Jt(_, _) => 3,
            Jf(_, _) => 3,
            Add(_, _, _) => 4,
            Mult(_, _, _) => 4,
            Mod(_, _, _) => 4,
            And(_, _, _) => 4,
            Or(_, _, _) => 4,
            Not(_, _) => 3,
            Rmem(_, _) => 3,
            Wmem(_, _) => 3,
            Call(_) => 2,
            Ret => 1,
            Out(_) => 2,
            In(_) => 2,
            Noop => 1,
            __Invalid => 1,
        }
    }

    /// Next pointer for branchings instructions
    pub fn next_possible_ip(&self) -> Vec<Val> {
        use Opcode::*;

        match self {
            Halt => vec![],
            Set(_, _) => vec![],
            Push(_) => vec![],
            Pop(_) => vec![],
            Eq(_, _, _) => vec![],
            Gt(_, _, _) => vec![],
            Jmp(a) => vec![*a],
            Jt(_, b) => vec![*b],
            Jf(_, b) => vec![*b],
            Add(_, _, _) => vec![],
            Mult(_, _, _) => vec![],
            Mod(_, _, _) => vec![],
            And(_, _, _) => vec![],
            Or(_, _, _) => vec![],
            Not(_, _) => vec![],
            Rmem(_, _) => vec![],
            Wmem(_, _) => vec![],
            Call(a) => vec![*a],
            Ret => vec![],
            Out(_) => vec![],
            In(_) => vec![],
            Noop => vec![],
            __Invalid => vec![],
        }
    }

    /// Return `Opcode)` decoded at `ip`
    pub fn fetch(memory: &[u16], ip: usize) -> Result<Opcode, Box<dyn std::error::Error>> {
        let instr_type = memory[ip];

        let opcode = match instr_type {
            0 => Opcode::Halt,
            1 => Opcode::Set(Val::new(memory[ip + 1]), Val::new(memory[ip + 2])),
            2 => Opcode::Push(Val::new(memory[ip + 1])),
            3 => Opcode::Pop(Val::new(memory[ip + 1])),
            4 => Opcode::Eq(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            5 => Opcode::Gt(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            6 => Opcode::Jmp(Val::new(memory[ip + 1])),
            7 => Opcode::Jt(Val::new(memory[ip + 1]), Val::new(memory[ip + 2])),
            8 => Opcode::Jf(Val::new(memory[ip + 1]), Val::new(memory[ip + 2])),
            9 => Opcode::Add(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            10 => Opcode::Mult(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            11 => Opcode::Mod(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            12 => Opcode::And(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            13 => Opcode::Or(
                Val::new(memory[ip + 1]),
                Val::new(memory[ip + 2]),
                Val::new(memory[ip + 3]),
            ),
            14 => Opcode::Not(Val::new(memory[ip + 1]), Val::new(memory[ip + 2])),
            15 => Opcode::Rmem(Val::new(memory[ip + 1]), Val::new(memory[ip + 2])),
            16 => Opcode::Wmem(Val::new(memory[ip + 1]), Val::new(memory[ip + 2])),
            17 => Opcode::Call(Val::new(memory[ip + 1])),
            18 => Opcode::Ret,
            19 => Opcode::Out(Val::new(memory[ip + 1])),
            20 => Opcode::In(Val::new(memory[ip + 1])),
            21 => Opcode::Noop,
            std::u16::MAX => Opcode::__Invalid,
            x => return Err(format!("Can't decode opcode {}", x).into()),
        };

        Ok(opcode)
    }

    pub fn disassemble(
        machine_code: &[u16],
        mut start: usize,
    ) -> Result<Vec<(usize, Opcode)>, Box<dyn std::error::Error>> {
        let mut instructions = Vec::new();

        let mut ptr = 0;
        while ptr < machine_code.len() {
            let instr = Opcode::fetch(&machine_code, ptr)?;

            let size = instr.size();
            instructions.push((start, instr));

            start += size;
            ptr += size;
        }

        Ok(instructions)
    }

    pub fn assemble(&self) -> Vec<u16> {
        use Opcode::*;

        match self {
            Halt => vec![0],
            Set(a, b) => vec![1, a.as_binary(), b.as_binary()],
            Push(a) => vec![2, a.as_binary()],
            Pop(a) => vec![3, a.as_binary()],
            Eq(a, b, c) => vec![4, a.as_binary(), b.as_binary(), c.as_binary()],
            Gt(a, b, c) => vec![5, a.as_binary(), b.as_binary(), c.as_binary()],
            Jmp(a) => vec![6, a.as_binary()],
            Jt(a, b) => vec![7, a.as_binary(), b.as_binary()],
            Jf(a, b) => vec![8, a.as_binary(), b.as_binary()],
            Add(a, b, c) => vec![9, a.as_binary(), b.as_binary(), c.as_binary()],
            Mult(a, b, c) => vec![10, a.as_binary(), b.as_binary(), c.as_binary()],
            Mod(a, b, c) => vec![11, a.as_binary(), b.as_binary(), c.as_binary()],
            And(a, b, c) => vec![12, a.as_binary(), b.as_binary(), c.as_binary()],
            Or(a, b, c) => vec![13, a.as_binary(), b.as_binary(), c.as_binary()],
            Not(a, b) => vec![14, a.as_binary(), b.as_binary()],
            Rmem(a, b) => vec![15, a.as_binary(), b.as_binary()],
            Wmem(a, b) => vec![16, a.as_binary(), b.as_binary()],
            Call(a) => vec![17, a.as_binary()],
            Ret => vec![18],
            Out(a) => vec![19, a.as_binary()],
            In(a) => vec![20, a.as_binary()],
            Noop => vec![21],
            __Invalid => vec![std::u16::MAX],
        }
    }

    pub fn assemble_vec(v: &[Opcode]) -> Vec<u16> {
        let mut machine_code = Vec::new();

        for opcode in v {
            machine_code.extend(&opcode.assemble());
        }

        machine_code
    }
}
