use colorgrad::Gradient;
use itertools::Itertools;
use regex::Regex;

use std::{collections::HashMap, fmt::Debug, str::FromStr};

use crate::{
    assembly::{Opcode, Val},
    emulator::{Function, StopVmState, Vm, VmState},
};
use std::{
    collections::{hash_map::DefaultHasher, BTreeMap, HashSet},
    hash::{Hash, Hasher},
};

fn log_map(value: usize, in_low: usize, in_high: usize, out_low: f32, out_high: f32) -> f32 {
    let value = (value as f32).log10();
    let max = (in_high as f32).log10();
    let min = (in_low as f32).log10();

    (value - min) / (max - min) * (out_high - out_low) + out_low
}

pub struct GameSolver {}

fn hash<T: Hash>(t: &T) -> u64 {
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);
    hasher.finish()
}

impl GameSolver {
    pub fn explore_maze(
        vm: &Vm,
        limit: &Option<Regex>,
    ) -> HashMap<u64, (Room, HashMap<String, u64>)> {
        let message = vm.get_messages().last().unwrap();
        let mut clusters: HashMap<String, Vec<String>> = HashMap::new();
        let room: Room = message.parse().expect("Missing room name (look)");
        let first_room = room.clone();

        let mut maze: HashMap<u64, (Room, HashMap<String, u64>)> = HashMap::new();

        // graph search datastructures
        let mut explored: HashSet<Room> = Default::default();
        let mut queue: BTreeMap<Room, Vm> = Default::default();
        queue.insert(room, vm.clone());

        let mut graphviz = String::from("digraph G {\n");

        while let Some((current_room, current_vm)) = queue.pop_first() {
            if explored.contains(&current_room) {
                continue;
            }

            if limit.is_some() && !limit.as_ref().unwrap().is_match(&current_room.name) {
                explored.insert(current_room);
                continue;
            }

            let from = hash(&current_room);

            let things = current_room.things.join(" ");

            let (color, penwidth) = match (current_room.things.is_empty(), current_vm.get_state()) {
                (_, VmState::Halted) => ("red", "3"),
                (false, _) => ("green", "3"),
                (true, _) => ("black", "1"),
            };

            let shape = if current_room == first_room {
                "box"
            } else {
                "ellipse"
            };

            let graphviz_room = format!(
                "{} [label=\"{} - {}: {}\",\n color = {},\n shape = {},\n penwidth = {}];\n",
                from,
                current_room.name,
                current_room.description.replace('\"', " "),
                things,
                color,
                shape,
                penwidth
            );

            #[allow(clippy::format_in_format_args)]
            clusters
                .entry(
                    current_room
                        .name
                        .replace(|c: char| !c.is_ascii_alphanumeric(), " ")
                        .split_whitespace()
                        .join("_"),
                )
                .or_default()
                .push(graphviz_room);

            let mut exits: HashMap<String, u64> = HashMap::new();
            for exit in &current_room.exits {
                let mut next_vm = current_vm.clone();
                next_vm.feed(exit).unwrap();
                let _ = next_vm
                    .run_until(StopVmState::new(&[
                        VmState::WaitingForInput,
                        VmState::Halted,
                        VmState::HitBreakPoint,
                    ]))
                    .unwrap();

                if next_vm.get_state() == VmState::Halted {
                    // TODO
                    //continue;
                }
                let message = next_vm.get_messages().last().unwrap();
                let new_room = match message.parse() {
                    Ok(l) => l,
                    Err(_) => Room {
                        name: "custom room".into(),
                        description: message.split_whitespace().join(" "),
                        exits: Vec::new(),
                        things: Vec::new(),
                    },
                };

                let to = hash(&new_room);

                graphviz.push_str(&format!("{} -> {} [label =\"{}\"];\n", from, to, exit));
                exits.insert(exit.clone(), to);

                if !explored.contains(&new_room) {
                    queue.insert(new_room, next_vm);
                }
            }

            maze.insert(from, (current_room.clone(), exits));
            explored.insert(current_room);
        }

        let colors = [
            "lightblue",
            "lightyellow",
            "darkseagreen1",
            "lightpink",
            "lightgray",
        ];
        for (idx, (cluster, rooms)) in clusters.iter().enumerate() {
            let color = colors[idx % colors.len()];
            graphviz.push_str(&format!("subgraph cluster_{} {{\n", cluster));
            graphviz.push_str("style = filled;\n");
            graphviz.push_str(&format!("bgcolor = {};\n", color));
            graphviz.push_str(&format!("label = \"{}\";\n", cluster));
            for room in rooms {
                graphviz.push_str(room);
            }

            graphviz.push_str("}\n");
        }

        graphviz.push_str("}\n\n");

        match std::fs::write("graphviz.dot", graphviz) {
            Ok(_) => (),
            Err(x) => println!("{:?}", x),
        }
        println!("\nSee ./graphviz.dot");

        println!("Finished exploring. List of rooms:");
        for room in &explored {
            println!("{}", room.name);
            for thing in &room.things {
                println!("- {}", thing);
            }
        }

        maze
    }

    pub fn trace_teleporter(vm: &Vm) {
        use Opcode::*;
        use Val::*;

        let mut vm = vm.clone();
        vm.set_traced_opcodes(Call(Invalid).discriminant());

        println!("You will need a lot of RAM for this...");

        //vm.set_fn_patching(true);
        let val = 1;
        dbg!(val);
        vm.set_register(7, val);

        let _ = vm.feed("use teleporter");

        let mut steps = 1_000_000;
        let chrono = std::time::Instant::now();
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
        dbg!(chrono.elapsed());

        dbg!(vm.get_trace_buffer().len());

        let counters = vm.get_trace_buffer().iter().sorted().counts();
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

        fn find_fn(functions: &[Function], offset: usize) -> Option<&Function> {
            functions
                .iter()
                .find(|f| f.start <= offset && offset <= f.end)
        }

        let min_calls = counters.iter().min_by_key(|(_, calls)| *calls).unwrap().1;
        let max_calls = counters.iter().max_by_key(|(_, calls)| *calls).unwrap().1;

        // draw edges
        for ((caller_offset, op, _), calls_count) in &counters {
            let Opcode::Call(target_offset) = op else {
                continue;
            };
            let caller_function = find_fn(&functions, *caller_offset);
            let caller_function = match caller_function {
                Some(f) => f,
                None => {
                    println!("can't find caller fn containing offset {}", caller_offset);
                    continue;
                }
            };
            let caller_name = format!("{}", caller_function.start);

            let target_offset = match target_offset {
                Val::Num(addr) => *addr,
                Val::Reg(_reg) => {
                    let addr = vm
                        .get_trace_buffer()
                        .iter()
                        .filter(|(offset, _op, _resolved_op)| *offset == *caller_offset)
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

            let target_function = find_fn(&functions, target_offset as usize)
                .unwrap_or_else(|| panic!("Can't find target function for call {:?}", op));
            let target_function_name = format!("{}", target_function.start);

            let gradient = colorgrad::preset::turbo();
            let t = log_map(*calls_count, *min_calls, *max_calls, 0., 1.);
            let color = gradient.at(t).to_css_hex();

            let edge_graphviz =
                        format!("\"{caller_name}\":\"{caller_offset}-op\":e -> \"{target_function_name}\":0:w [color=\"{color}\", taillabel=\"{calls_count}\"]\n");
            graphviz.push_str(&edge_graphviz);
        }

        graphviz.push_str("}\n");

        std::fs::write("graphviz.dot", graphviz).unwrap();
        println!("See graphviz.dot");
    }

    pub fn brute_force_fn_6027(vm: &Vm) {
        use Opcode::*;
        use Val::*;

        let mut last_messages = HashMap::new();
        for val in 0..=u16::MAX {
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

    pub fn solve_vault(vm: &Vm) {
        for val in 0..u16::MAX {
            let mut game = Game::from_vm(vm);
            game.vm.mem_set(3952, val);
            game.action("east").unwrap();

            let m = game.message.as_ref().unwrap();
            if m.contains("The orb shatters") || m.contains("The orb evaporates out of your hands.")
            {
                // lost
            } else {
                dbg!(m);
                dbg!(val);
            }
        }

        //let game = Game::from_vm(vm);
        //inner(game, Vec::new(), 20);
        //println!("no solution");
        //fn inner(game: Game, path: Vec<String>, depth: i32) {
        //    if depth == 0 {
        //        // early stop
        //        return;
        //    }
        //    let m = game.message.as_ref().unwrap();
        //    if m.contains("You hear a click from the vault door.") {
        //        // WIN
        //        println!("{:?}", path);
        //        panic!();
        //        return;
        //    }
        //    if m.contains("The orb shatters!") || m.contains("evaporates") {
        //        // lost
        //        return;
        //    }
        //    for exit in &game.room.exits {
        //        // recurse
        //        let mut new_game = game.clone();
        //        new_game.action(&exit).unwrap();
        //        let mut new_path = path.clone();
        //        new_path.push(exit.to_string());
        //        inner(new_game, new_path, depth - 1);
        //    }
        //}
    }
}

#[derive(Clone)]
struct Game {
    vm: Vm,
    room: Room,
    message: Option<String>,
}

impl Game {
    pub fn from_vm(vm: &Vm) -> Self {
        let mut vm = vm.clone();

        let (message, room) = vm.feed_and_parse("look").unwrap();

        Self {
            vm: vm.clone(),
            room,
            message,
        }
    }

    pub fn action(&mut self, action: &str) -> Result<(), Box<dyn std::error::Error>> {
        (self.message, self.room) = self.vm.feed_and_parse(action)?;

        Ok(())
    }
}

enum Node {
    Number(i32),
    Operation(Operation),
}
impl FromStr for Node {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<Operation>()
            .map(|op| Node::Operation(op))
            .or_else(|_| Ok(Node::Number(s.parse::<i32>().map_err(|_| ())?)))
    }
}

enum Operation {
    Plus,
    Minus,
    Mult,
    Div,
}

impl FromStr for Operation {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "+" => Ok(Operation::Plus),
            "-" => Ok(Operation::Minus),
            "*" => Ok(Operation::Mult),
            "/" => Ok(Operation::Div),
            _ => Err(()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Room {
    pub name: String,
    pub description: String,
    pub things: Vec<String>,
    pub exits: Vec<String>,
}

impl Hash for Room {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.description.hash(state);
        self.things.hash(state);
        self.exits.hash(state);
    }
}

impl FromStr for Room {
    type Err = Box<dyn std::error::Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re_name = Regex::new(r"== (.+?) ==\n(.+?)\n")?;
        let (name, mut description) = {
            let caps = re_name.captures(s).ok_or("No room name")?;

            (
                caps.get(1).unwrap().as_str().to_string(),
                caps.get(2).unwrap().as_str().to_string(),
            )
        };

        if description.contains("You are in a grid of rooms that control the door to the vault.") {
            description.push_str(s.lines().nth(5).unwrap());
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
        let things = get_things(s);

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
        let exits = get_exits(s);

        let room = Room {
            description,
            name,
            things,
            exits,
        };

        Ok(room)
    }
}

impl Room {}
