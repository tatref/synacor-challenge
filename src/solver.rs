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
