use itertools::Itertools;
use regex::Regex;

use std::{collections::HashMap, fmt::Debug};

use crate::{
    assembly::{Opcode, Val},
    emulator::{Function, StopVmState, Vm, VmState},
};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashSet},
    hash::{Hash, Hasher},
};

pub struct GameSolver {}

impl GameSolver {
    pub fn explore_maze(vm: &Vm) {
        let message = vm.get_messages().last().unwrap();
        let room = Room::from(message).expect("Missing room name (look)");
        let first_room = room.clone();

        let mut explored: HashSet<Room> = Default::default();
        let mut queue: BTreeMap<Room, Vm> = Default::default();
        queue.insert(room, vm.clone());

        let mut graphviz = String::from("digraph G {\n");

        while let Some((current_room, current_vm)) = queue.pop_first() {
            if explored.contains(&current_room) {
                continue;
            }

            //dbg!(explored.len(), queue.len());
            //println!("Exploring {}", current_room.name);

            for exit in &current_room.exits {
                let mut vm = current_vm.clone();
                vm.feed(exit).unwrap();
                let _ = vm
                    .run_until(StopVmState::new(&[
                        VmState::WaitingForInput,
                        VmState::Halted,
                        VmState::HitBreakPoint,
                    ]))
                    .unwrap();

                if vm.get_state() == VmState::Halted {
                    // TODO
                    //continue;
                }
                let message = vm.get_messages().last().unwrap();
                let new_room = match Room::from(message) {
                    Ok(l) => l,
                    Err(_) => Room {
                        name: "custom room".into(),
                        description: message.to_string(),
                        exits: Vec::new(),
                        things: Vec::new(),
                    },
                };

                fn hash_string(input: &str) -> u64 {
                    let mut hasher = DefaultHasher::new();
                    input.hash(&mut hasher);
                    hasher.finish()
                }
                let from = hash_string(&format!(
                    "{}{}",
                    current_room.name, current_room.description
                ));
                let to = hash_string(&format!("{}{}", new_room.name, new_room.description));

                let things = current_room.things.join(" ");
                //let color = if current_room.things.is_empty() {
                //    "black"
                //} else {
                //    "green"
                //};

                let color = match (current_room.things.is_empty(), vm.get_state()) {
                    (_, VmState::Halted) => "red",
                    (false, _) => "green",
                    (true, _) => "black",
                };

                let shape = if current_room == first_room {
                    "Mdiamond"
                } else {
                    "ellipse"
                };
                graphviz.push_str(&format!("{} -> {} [label =\"{}\"];\n", from, to, exit));

                #[allow(clippy::format_in_format_args)]
                graphviz.push_str(&format!(
                    "{} [label=\"{} - {}: {}\", color = {}, shape = {}];\n",
                    from,
                    current_room.name,
                    current_room.description.replace('\"', ""),
                    things,
                    color,
                    shape
                ));

                if explored.contains(&new_room) {
                    continue;
                }

                queue.insert(new_room, vm.clone());
            }

            explored.insert(current_room);
        }

        println!("Finished exploring. List of rooms:");
        for room in &explored {
            println!("{}", room.name);
            for thing in &room.things {
                println!("- {}", thing);
            }
        }

        graphviz.push_str("}\n\n");

        match std::fs::write("graphviz.dot", graphviz) {
            Ok(_) => (),
            Err(x) => println!("{:?}", x),
        }
        println!("\nSee ./graphviz.dot");
    }

    pub fn trace_teleporter(vm: &Vm) -> Vm {
        use Opcode::*;
        use Val::*;

        let val = 1;
        //for val in 0..u16::MAX {
        dbg!(val);
        let mut vm = vm.clone();
        vm.set_traced_opcodes(Call(Invalid).discriminant());

        vm.set_fn_patching(true);
        vm.set_register(7, val);

        let _ = vm.feed("use teleporter");

        let mut steps = 1_000_000_000;
        while vm.get_state() == VmState::Running {
            match vm.step() {
                Ok(_) => (),
                Err(_e) => break,
            }
            steps -= 1;
            if steps == 0 {
                println!("early stop {}", val);
                break;
            }
        }

        dbg!(vm.get_trace_buffer().len());

        let x = vm
            .get_trace_buffer()
            .iter()
            .filter(|(_offset, op, _resolved_op)| match op {
                Opcode::Call(Num(6026)) => true,
                _ => false,
            })
            .next();
        dbg!(x);

        let counters = vm.get_trace_buffer().iter().counts();
        let sorted_counters: Vec<_> = counters
            .iter()
            .map(|((caller_offset, op, resolved_op), count)| {
                (count, caller_offset, op, resolved_op)
            })
            .sorted()
            .collect();

        let functions: Vec<Function> = sorted_counters
            .iter()
            .map(|(_count, _caller_offset, call_op, resolved_op)| {
                let addr = match call_op {
                    Call(Val::Invalid) => unreachable!("invalid value"),
                    Call(Val::Num(addr)) => *addr,
                    Call(Val::Reg(_reg)) => {
                        match resolved_op.expect("resolved op is None but should be Some") {
                            Call(Num(addr)) => addr,
                            _ => unreachable!(),
                        }
                    }
                    _ => unimplemented!(),
                };

                vm.disassemble_function(addr as usize).unwrap()
            })
            .sorted()
            .dedup()
            .collect();

        let mut graphviz = String::new();
        graphviz.push_str(
            r#"
digraph g {
fontname="Helvetica,Arial,sans-serif"
node [fontname="Helvetica,Arial,sans-serif"]
edge [fontname="Helvetica,Arial,sans-serif"]

graph [
    fontsize=30
    labelloc="t"
    label=""
    splines=true
    overlap=false
    rankdir = "LR"
];

node [fontname="Helvetica,Arial,sans-serif"]
edge [fontname="Helvetica,Arial,sans-serif"]

"missing" [];

"#,
        );

        for function in &functions {
            let function_graphviz = function.graphviz();
            graphviz.push_str(&function_graphviz);
            graphviz.push('\n');
        }

        // draw edges
        for function in &functions {
            let name = format!("{}", function.start);

            for (offset, op) in function.get_code().iter().enumerate() {
                let offset = offset + function.start;
                if let Opcode::Call(dest) = op {
                    let addr = match dest {
                        Val::Num(addr) => *addr,
                        Val::Reg(_reg) => {
                            // TODO: trace_buffer
                            let addr = vm
                                .get_trace_buffer()
                                .iter()
                                .filter(|(_offset, _op, _resolved_op)| *_offset == offset)
                                .map(|(_, _, op)| match op.unwrap() {
                                    Call(Num(addr)) => addr,
                                    _ => unimplemented!(),
                                })
                                .next();

                            // 0 = unknown
                            addr.unwrap_or(0)
                        }
                        _ => todo!(),
                    };

                    let target_function = functions
                        .iter()
                        .find(|f| f.start <= addr as usize && (addr as usize) <= f.end);

                    let target_function_name = match target_function {
                        Some(f) => format!("{}", f.start),
                        None => "missing".to_string(),
                    };

                    let edge_graphviz =
                        format!("\"{name}\":\"{offset}\":w -> \"{target_function_name}\":0\n");
                    graphviz.push_str(&edge_graphviz);
                }
            }
        }

        graphviz.push_str("}\n");

        std::fs::write("graphviz.txt", graphviz).unwrap();

        panic!();
    }
}

pub fn brute_force_fn_2027(vm: &Vm) {
    use Opcode::*;
    use Val::*;

    let mut last_messages = HashMap::new();
    for val in 0..=u16::MAX {
        //for val in 0..u16::MAX {
        dbg!(val);
        let mut vm = vm.clone();
        vm.set_traced_opcodes(Call(Invalid).discriminant());

        vm.set_fn_patching(true);
        vm.set_register(7, val);

        let _ = vm.feed("use teleporter");

        // without 2027 patching, we need a lot of instructions
        let mut steps = 1_000_000_000;
        while vm.get_state() == VmState::Running {
            match vm.step() {
                Ok(_) => (),
                Err(_e) => break,
            }
            steps -= 1;
            if steps == 0 {
                println!("early stop {}", val);
                break;
            }
        }

        vm.run_until(StopVmState::new(&[VmState::WaitingForInput]))
            .unwrap();

        let last_message = vm.get_messages().last().unwrap();
        last_messages.insert(last_message.clone(), val);

        if last_message.contains("Miscalibration detected!  Aborting teleportation!") {
            continue;
        } else {
            dbg!("Found", val, &last_message);
        }
    }

    dbg!(&last_messages);
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub things: Vec<String>,
    pub exits: Vec<String>,
}

impl Room {
    pub fn from(raw: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let re_name = Regex::new(r"== (.+?) ==\n(.+?)\n").unwrap();
        let (name, mut description) = {
            let caps = re_name.captures(raw).ok_or("No room name")?;

            (
                caps.get(1).unwrap().as_str().to_string(),
                caps.get(2).unwrap().as_str().to_string(),
            )
        };

        if description.contains("You are in a grid of rooms that control the door to the vault.") {
            description.push_str(raw.lines().nth(5).unwrap());
            description = description.replace('\n', " ");
        }

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

        let room = Room {
            description,
            name,
            things,
            exits,
        };

        Ok(room)
    }
}
