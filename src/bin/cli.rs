use std::env;
use std::fs::File;
use std::io::Read;

use static_analysis::domains::*;
use static_analysis::semantics::*;
use static_analysis::parsing::*;
use static_analysis::cfg::*;


fn execute<T: AbstractDomain>(prog: &str, init_state: Option<State<T>>) -> String {
	let mut graph = parse_program::<T>(prog).unwrap().unwrap();

	let state = init_state.unwrap_or(State::new());

    let out = execute_program(prog, &mut graph, state).unwrap();

    out
}

fn main() {
    let mut fd = None;
    let mut domain = "sign";
    let mut is_file = false;
    let mut is_domain = false;
    for arg in env::args() {
        if is_file {
            fd = Some(File::open(&arg)
                .map_err(|e| e.to_string())
                .expect("Failed to open file"));
            is_file = false;
        }
        if is_domain {
            domain = match arg.as_str() {
                "sign" => "sign",
                "interval" => "interval",
                _ => panic!("Unknown abstract domain: {arg}\nSupported: 'sign', 'interval'")
            };
            is_domain = false;
        }

        if arg == "--file" || arg == "-f" {
            is_file = true;
        }
        if arg == "--domain" || arg == "-d" {
            is_domain = true;
        }
    }
 
    let mut prog = String::new();
    let _ = match fd {
        Some(mut src_file) => src_file.read_to_string(&mut prog), 
        None => std::io::stdin().read_to_string(&mut prog)
    };

    interval::set_bounds(-64, 64);
    let out = match domain {
        "sign" => execute::<Sign>(&prog, None),
        "interval" => execute::<Interval>(&prog, None),
        _ => panic!("Not possible to reach this point"),
    };

    println!("{out}");
}
