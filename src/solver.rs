use regex::Regex;

use crate::emulator::{Opcode, Val, Vm, VmState};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashMap, HashSet},
    hash::{Hash, Hasher},
};

pub struct GameSolver {}

impl GameSolver {
    pub fn explore_maze(vm: &Vm) {
        let message = vm.get_messages().last().unwrap();
        let level = Level::from(message).unwrap();
        let first_level = level.clone();

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

                let shape = if current_level == first_level {
                    "Mdiamond"
                } else {
                    "ellipse"
                };
                graphviz.push_str(&format!("{} -> {} [label =\"{}\"];\n", from, to, exit));

                #[allow(clippy::format_in_format_args)]
                graphviz.push_str(&format!(
                    "{} [label=\"{} - {}: {}\", color = {}, shape = {}];\n",
                    from,
                    current_level.name,
                    current_level.description.replace("\"", ""),
                    things,
                    color,
                    shape
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

        graphviz.push_str("}\n\n");

        match std::fs::write("graphviz.dot", graphviz) {
            Ok(_) => (),
            Err(x) => println!("{:?}", x),
        }
        println!("./graphviz.dot");
    }

    pub fn trace_teleporter(vm: &Vm) {
        let mut vm = vm.clone();
        vm.set_patching(false);

        let val = 1;
        vm.set_register(7, val);

        let traced = Opcode::Jmp(Val::Invalid).discriminant()
            | Opcode::Ret.discriminant()
            | Opcode::Call(Val::Invalid).discriminant();
        vm.set_traced_opcodes(traced);

        let _ = vm.feed("use teleporter");

        let mut steps = 10000;
        while vm.get_state() == VmState::Running {
            vm.step().unwrap();
            steps -= 1;
            if steps == 0 {
                //println!("early stop {}", val);
                break;
            }
        }
        if vm.get_state() != VmState::Running {
            dbg!(&vm.get_messages().last());
        }

        vm.set_traced_opcodes(0);

        fn count_occurrences<T: Eq + std::hash::Hash + Copy>(input: &[T]) -> Vec<(T, usize)> {
            let mut counts = HashMap::new();

            for &item in input {
                let count = counts.entry(item).or_insert(0);
                *count += 1;
            }

            let mut occurrences: Vec<(T, usize)> = counts.into_iter().collect();
            occurrences.sort_by(|a, b| b.1.cmp(&a.1));
            occurrences
        }
        fn top_n_occurrences<T: Eq + std::hash::Hash + Copy>(
            input: &[T],
            n: usize,
        ) -> Vec<(T, usize)> {
            let occurrences = count_occurrences(input);
            occurrences.into_iter().take(n).collect()
        }

        let top_occurrences = top_n_occurrences(vm.get_trace_buffer(), 10);
        for (opcode, count) in top_occurrences {
            println!("opcode: {:?}, Occurrences: {}", opcode, count);
        }

        //self.build_call_graph(&vm, vm.get_trace_buffer());
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
        let (name, mut description) = {
            let caps = re_name.captures(raw).ok_or("No level name")?;

            (
                caps.get(1).unwrap().as_str().to_string(),
                caps.get(2).unwrap().as_str().to_string(),
            )
        };

        if description.contains("You are in a grid of rooms that control the door to the vault.") {
            description.push_str(&raw.lines().nth(5).unwrap());
            description = description.replace("\n", " ");
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

        let level = Level {
            description,
            name,
            things,
            exits,
        };

        Ok(level)
    }
}
