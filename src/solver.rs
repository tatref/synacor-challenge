use itertools::Itertools;
use regex::Regex;

use crate::{
    assembly::{Opcode, Val},
    emulator::{StopVmState, Vm, VmState},
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

    pub fn trace_teleporter(vm: &Vm) {
        use Opcode::*;
        use Val::*;

        //for val in 43000..u16::MAX {
        let val = 1;
        dbg!(val);
        let mut vm = vm.clone();
        vm.set_traced_opcodes(Call(Invalid).discriminant());

        //vm.set_fn_patching(true);
        vm.set_register(7, val);

        let _ = vm.feed("use teleporter");

        let mut steps = 100_000_000;
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

        //Vm::pretty_print_dis(&vm.get_trace_buffer());
        dbg!(&vm.get_messages().last());

        dbg!(vm.get_trace_buffer().len());

        let counters = vm.get_trace_buffer().iter().counts();
        let v: Vec<_> = counters
            .iter()
            .map(|((offset, op), count)| (count, offset, op))
            .sorted()
            .collect();

        dbg!(v);

        panic!();
    }
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
