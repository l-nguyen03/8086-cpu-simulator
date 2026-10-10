use std::env;

pub struct Args {
    pub exec: bool,
    pub in_path: String,
    pub out_path: Option<String>,
}

impl Args {
    pub fn parse() -> Args {
        let mut args = env::args();
        let program = args.next().unwrap();

        let mut exec = false;
        let mut paths = Vec::with_capacity(2);

        for arg in args {
            if arg == "--exec" {
                exec = true;
            } else {
                paths.push(arg);
            }
        }

        let Some(mut in_path) = paths.pop() else {
            eprintln!("Usage: {program} [--exec] <binary-file> [out-file]");
            std::process::exit(1);
        };

        let mut out_path: Option<String> = paths.pop();
        if let Some(path) = out_path.as_mut() {
            std::mem::swap(path, &mut in_path)
        }

        Args {
            exec,
            in_path,
            out_path,
        }
    }
}
