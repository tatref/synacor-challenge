use regex::Regex;

use crate::emulator::{Opcode, Val, Vm, VmState};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
};

pub struct GameSolver {}

impl GameSolver {
    pub fn new() -> Self {
        Self {}
    }

    pub fn explore_maze(&self, vm: &Vm) {
        let message = vm.get_messages().last().unwrap();
        let level = Level::from(message).unwrap();

        let mut explored: HashSet<Level> = Default::default();
        let mut queue: BTreeMap<Level, Vm> = Default::default();
        queue.insert(level, vm.clone());

        let mut graphviz = String::from("digraph G {\n");

        while let Some((current_level, current_vm)) = queue.pop_first() {
            if explored.contains(&current_level) {
                continue;
            }

            //dbg!(explored.len(), queue.len());
            //println!("Exploring {}", current_level.name);

            for exit in &current_level.exits {
                let mut vm = current_vm.clone();
                vm.feed(exit).unwrap();
                vm.run();

                if vm.get_state() == VmState::Halted {
                    continue;
                }
                let message = vm.get_messages().last().unwrap();
                let new_level = match Level::from(message) {
                    Ok(l) => l,
                    Err(_) => Level {
                        name: "custom level".into(),
                        description: message.to_string(),
                        exits: Vec::new(),
                        things: Vec::new(),
                    },
                };

                //println!("exit {} => {}", exit, new_level.name);
                fn hash_string(input: &str) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    input.hash(&mut hasher);
                    hasher.finish()
                }
                let from = hash_string(&format!(
                    "{}{}",
                    current_level.name, current_level.description
                ));
                let to = hash_string(&format!("{}{}", new_level.name, new_level.description));
                let things = current_level.things.join(" ");
                let color = if current_level.things.is_empty() {
                    "black"
                } else {
                    "red"
                };

                graphviz.push_str(&format!("{} -> {} [label =\"{}\"];\n", from, to, exit));
                graphviz.push_str(&format!(
                    "{} [label=\"{}\", color = {}];\n",
                    from,
                    format!("{}: {}", current_level.name, things,),
                    color
                ));

                if explored.contains(&new_level) {
                    continue;
                }

                queue.insert(new_level, vm.clone());
            }

            explored.insert(current_level);
        }

        println!("Finished exploring");
        for level in &explored {
            println!("{}", level.name);
            for thing in &level.things {
                println!("- {}", thing);
            }
        }

        graphviz.push_str("\n}\n");
        println!("{}", graphviz);
    }

    pub fn build_call_graph(&self, vm: &Vm, calls: &[(usize, Opcode)]) {
        // https://graphviz.org/Gallery/directed/datastruct.html
        let mut graph = r#"
        digraph g {
        fontname="Helvetica,Arial,sans-serif"
        node [fontname="Helvetica,Arial,sans-serif"]
        edge [fontname="Helvetica,Arial,sans-serif"]
        graph [
        rankdir = "LR"
        ];
        node [
        fontsize = "16"
        shape = "ellipse"
        ];
        edge [
        ];
        "#
        .to_string();

        #[derive(Clone)]
        struct Function {
            start: u16,
            end: Option<u16>,
            callers: HashMap<u16, usize>,
        }

        let mut functions: HashMap<u16, Function> = HashMap::new();
        let mut current_function: Option<u16> = None;

        for (ip, opcode) in calls {
            match opcode {
                Opcode::Call(a) => match a {
                    Val::Invalid => (),
                    Val::Reg(_) => (),
                    Val::Num(a) => {
                        *functions
                            .entry(*a)
                            .or_insert(Function {
                                start: *a,
                                end: None,
                                callers: HashMap::new(),
                            })
                            .callers
                            .entry(*ip as u16)
                            .or_insert(0) += 1;

                        current_function = Some(*a);
                    }
                },
                Opcode::Ret => {
                    functions
                        .entry(current_function.unwrap())
                        .and_modify(|f| f.end = Some(*ip as u16));
                }
                Opcode::Jmp(_a) => (),
                Opcode::Jf(_a, _b) => (),
                Opcode::Jt(_a, _b) => (),
                _ => (),
            }
        }

        for f in &functions {}
        //"node0" [
        //label = "<f0> 0x10ba8| <f1>"
        //shape = "record"
        //];
        //"node1" [
        //label = "<f0> 0xf7fc4380| <f1> abcd | <f2> |-1"
        //shape = "record"
        //];
        //"node2" [
        //label = "<f0> 0xf7fc44b8| | |2"
        //shape = "record"
        //];
        //
        //"node0":f0 -> "node1":f0 [
        //id = 0
        //];
        //"node0":f1 -> "node2":f0 [
        //id = 1
        //];

        graph.push_str("}\n");
    }

    pub fn solve_teleporter(&self, vm: &Vm) {
        let mut vm = vm.clone();

        let val = 1;
        vm.set_register(7, val);

        let traced = Opcode::Jmp(Val::Invalid).discriminant()
            | Opcode::Ret.discriminant()
            | Opcode::Call(Val::Invalid).discriminant();
        vm.set_traced_opcodes(traced);

        let _ = vm.feed("use teleporter");

        let mut steps = 100;
        while vm.get_state() == VmState::Running {
            vm.step().unwrap();
            steps -= 1;
            if steps == 0 {
                println!("early stop {}", val);
                break;
            }
        }

        vm.set_traced_opcodes(0);

        let mut map = HashMap::new();
        for e in vm.get_trace_buffer() {
            *map.entry(e).or_insert(0) += 1;
        }

        for x in map.iter() {
            println!("{:?}", x);
        }

        for traces in vm.get_trace_buffer() {
            println!("{:?}", traces);
        }

        self.build_call_graph(&vm, vm.get_trace_buffer());
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Level {
    pub name: String,
    pub description: String,
    pub things: Vec<String>,
    pub exits: Vec<String>,
}

impl Level {
    pub fn from(raw: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let re_name = Regex::new(r"== (.+?) ==\n(.+?)\n").unwrap();
        let (name, description) = {
            let caps = re_name.captures(raw).ok_or("No level name")?;

            (
                caps.get(1).unwrap().as_str().to_string(),
                caps.get(2).unwrap().as_str().to_string(),
            )
        };

        fn get_things(raw: &str) -> Vec<String> {
            let re_things = Regex::new(r"Things of interest here:\n([^\n]+\n)+").unwrap();
            let things_str = match re_things.captures(raw) {
                Some(x) => x.get(0).unwrap().as_str().to_string(),
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
            let re_exits = Regex::new(r"(?s)There \w+ \d+ exits?:\n([^\n]+\n)+").unwrap();
            let exits_str = match re_exits.captures(raw) {
                Some(x) => x.get(0).unwrap().as_str().to_string(),
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
            description,
            name,
            things,
            exits,
        };

        Ok(level)
    }
}
