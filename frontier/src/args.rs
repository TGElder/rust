pub enum Args {
    New {
        power: usize,
        seed: u64,
        threads: usize,
        reveal_all: bool,
    },
    Load {
        path: String,
        threads: usize,
    },
}

#[allow(clippy::comparison_chain)]
impl Args {
    pub fn new(args: Vec<String>) -> Args {
        if args.len() > 2 {
            Args::New {
                power: args[1].parse().unwrap(),
                seed: args[2].parse().unwrap(),
                threads: args[3].parse().unwrap(),
                reveal_all: args.contains(&"-r".to_string()),
            }
        } else if args.len() == 2 {
            Args::Load {
                path: args[1].clone(),
                threads: args[2].parse().unwrap(),
            }
        } else {
            panic!("Invalid command line arguments");
        }
    }
}
