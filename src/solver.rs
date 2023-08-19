use regex::Regex;

use crate::emulator::Vm;
use std::collections::HashSet;

pub struct GameSolver {
    //game: Game,
    pub levels: Vec<Level>,
}

impl GameSolver {
    pub fn new() -> Self {
        Self { levels: Vec::new() }
    }

    pub fn explore_maze(&self, vm: &Vm, name: &str) {
        let vm = vm.clone();
        let message = vm.get_messages().last().unwrap();

        let level = Level::from(&message).unwrap();
        dbg!(level);
    }
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
            let caps = re_name.captures(raw).ok_or_else(|| "No level name")?;

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
            let re_exits = Regex::new(r"(?s)There are \d+ exits:\n([^\n]+\n)+").unwrap();
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
