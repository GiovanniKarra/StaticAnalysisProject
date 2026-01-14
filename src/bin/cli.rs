use std::env;
use std::fs::File;
use std::io::Read;

use static_analysis::domains::*;
use static_analysis::semantics::State;
use static_analysis::cfg::execute;


fn main() {
    let mut fd = None;
    let mut domain = "interval";
    let mut narrow = None;
    let mut widen = None;
    let mut is_file = false;
    let mut is_domain = false;
    let mut is_narrow = false;
    let mut is_widen = false;
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
        if is_widen {
            widen = Some(arg.parse::<u32>().expect("Widening delay should be an unsigned integer"));
            is_widen = false;
        }
        if is_narrow {
            narrow = Some(arg.parse::<u32>().expect("Narrowing steps should be an unsigned integer"));
            is_narrow = false;
        }

        if arg == "--file" || arg == "-f" {
            is_file = true;
        }
        if arg == "--domain" || arg == "-d" {
            is_domain = true;
        }
        if arg == "--widen" || arg == "-w" {
            is_widen = true;
        }
        if arg == "--narrow" || arg == "-n" {
            is_narrow = true;
        }
    }
 
    let mut prog = String::new();
    let _ = match fd {
        Some(mut src_file) => src_file.read_to_string(&mut prog), 
        None => std::io::stdin().read_to_string(&mut prog)
    };

    interval::set_bounds(-10000, 10000);
    let out = match domain {
        "sign" => execute::<Sign>(&prog, None, widen, narrow),
        "interval" => {
            let idx = prog.find("===").unwrap_or(0);
            let (red_prog, init_state) = if idx == 0 {
                (&prog[..], None)
            } else {
                (
                    &prog[idx+3..],
                    Some(State::from_str(&prog[..idx]).unwrap_or(State::new()))
                )
            };
            execute::<Interval>(red_prog, init_state, widen, narrow)
        },
        _ => panic!("Not possible to reach this point"),
    };

    println!("{out}");
}
