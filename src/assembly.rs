use serde::{Deserialize, Serialize};
use std::fmt;

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
            dbg!(l_par);
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

    pub fn size(&self) -> usize {
        match self {
            Opcode::Halt => 1,
            Opcode::Set(_, _) => 3,
            Opcode::Push(_) => 2,
            Opcode::Pop(_) => 2,
            Opcode::Eq(_, _, _) => 4,
            Opcode::Gt(_, _, _) => 4,
            Opcode::Jmp(_) => 2,
            Opcode::Jt(_, _) => 3,
            Opcode::Jf(_, _) => 3,
            Opcode::Add(_, _, _) => 4,
            Opcode::Mult(_, _, _) => 4,
            Opcode::Mod(_, _, _) => 4,
            Opcode::And(_, _, _) => 4,
            Opcode::Or(_, _, _) => 4,
            Opcode::Not(_, _) => 3,
            Opcode::Rmem(_, _) => 3,
            Opcode::Wmem(_, _) => 3,
            Opcode::Call(_) => 2,
            Opcode::Ret => 1,
            Opcode::Out(_) => 2,
            Opcode::In(_) => 2,
            Opcode::Noop => 1,
        }
    }

    /// Next pointer for branchings instructions
    pub fn next_possible_ip(&self) -> Vec<Val> {
        match self {
            Opcode::Halt => vec![],
            Opcode::Set(_, _) => vec![],
            Opcode::Push(_) => vec![],
            Opcode::Pop(_) => vec![],
            Opcode::Eq(_, _, _) => vec![],
            Opcode::Gt(_, _, _) => vec![],
            Opcode::Jmp(a) => vec![*a],
            Opcode::Jt(_, b) => vec![*b],
            Opcode::Jf(_, b) => vec![*b],
            Opcode::Add(_, _, _) => vec![],
            Opcode::Mult(_, _, _) => vec![],
            Opcode::Mod(_, _, _) => vec![],
            Opcode::And(_, _, _) => vec![],
            Opcode::Or(_, _, _) => vec![],
            Opcode::Not(_, _) => vec![],
            Opcode::Rmem(_, _) => vec![],
            Opcode::Wmem(_, _) => vec![],
            Opcode::Call(a) => vec![*a],
            Opcode::Ret => vec![],
            Opcode::Out(_) => vec![],
            Opcode::In(_) => vec![],
            Opcode::Noop => vec![],
        }
    }

    pub fn to_machine_code(&self) -> Vec<u16> {
        match self {
            Opcode::Halt => vec![0],
            Opcode::Set(a, b) => vec![1, a.as_binary(), b.as_binary()],
            Opcode::Push(_) => todo!(),
            Opcode::Pop(_) => todo!(),
            Opcode::Eq(a, b, c) => vec![4, a.as_binary(), b.as_binary(), c.as_binary()],
            Opcode::Gt(_, _, _) => todo!(),
            Opcode::Jmp(a) => vec![6, a.as_binary()],
            Opcode::Jt(a, b) => vec![7, a.as_binary(), b.as_binary()],
            Opcode::Jf(a, b) => vec![8, a.as_binary(), b.as_binary()],
            Opcode::Add(a, b, c) => vec![9, a.as_binary(), b.as_binary(), c.as_binary()],
            Opcode::Mult(_, _, _) => todo!(),
            Opcode::Mod(_, _, _) => todo!(),
            Opcode::And(_, _, _) => todo!(),
            Opcode::Or(_, _, _) => todo!(),
            Opcode::Not(_, _) => todo!(),
            Opcode::Rmem(_, _) => todo!(),
            Opcode::Wmem(_, _) => todo!(),
            Opcode::Call(a) => vec![17, a.as_binary()],
            Opcode::Ret => vec![18],
            Opcode::Out(_) => todo!(),
            Opcode::In(_) => todo!(),
            Opcode::Noop => vec![21],
        }
    }

    pub fn vec_to_machine_code(v: &[Opcode]) -> Vec<u16> {
        let mut machine_code = Vec::new();

        for opcode in v {
            machine_code.extend(&opcode.to_machine_code());
        }

        machine_code
    }
}
