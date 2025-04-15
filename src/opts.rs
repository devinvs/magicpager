use std::path::PathBuf;

fn version() {
    eprintln!("mp 0.0.1");
    std::process::exit(0);
}

fn usage(code: i32) {
    eprintln!("Usage: mp [OPTION]... [FILE]");
    eprintln!("       mp [OPTION]... -- [COMMAND]");
    eprintln!("Display the output of a file or command in the terminal.");
    eprintln!("Update the output on events selected by options.\n");

    eprintln!("Options:");
    eprintln!("  -0, --never      never update");
    eprintln!("  -t, --time=n     update every n seconds");
    eprintln!("  -f, --file=f     update when file f changes (default when file specified)");
    eprintln!("  -d, --dir=d      update when any file in dir d changes");
    eprintln!("  -s, --size       update when the terminal size changes\n");

    eprintln!("  -e, --errexit    exit if command has a non-zero exit");
    eprintln!("  --diff           highlight changes between updates\n");

    eprintln!("  -h, --help       display this help message");
    eprintln!("  --version        display the program version");

    std::process::exit(code);
}

#[derive(Clone)]
pub struct Options {
    pub time: Option<f64>,
    pub files: Vec<PathBuf>,
    pub size: bool,
    pub errexit: bool,
    pub diff: bool,
    pub never: bool,
    pub file: Option<PathBuf>,
    pub cmd: Option<String>,
}

pub fn parse_opts() -> Options {
    let mut opts = Options {
        time: None,
        files: vec![],
        size: false,
        errexit: false,
        diff: false,
        never: false,
        file: None,
        cmd: None,
    };

    let mut args = std::env::args();
    args.next().unwrap();
    while let Some(arg) = args.next() {
        // the basic matches
        match arg.as_str() {
            "-h" | "--help" => usage(0),
            "--version" => version(),
            "-0" | "--never" => {
                opts.never = true;
                continue;
            }
            "-t" | "--time" => {
                if opts.time.is_some() {
                    eprintln!("time option specified multiple times\n");
                    usage(1);
                }

                if let Some(Ok(t)) = args.next().map(|s| s.parse::<f64>()) {
                    opts.time = Some(t);
                    continue;
                } else {
                    eprintln!("numeric value expected for time argument\n");
                    usage(1);
                }
            }
            "-f" | "--file" => {
                if let Some(arg) = args.next() {
                    opts.files.push(arg.into());
                    continue;
                } else {
                    eprintln!("argument expected for file option\n");
                    usage(1);
                }
            }
            "-d" | "--dir" => {
                if let Some(arg) = args.next() {
                    opts.files.push(arg.into());
                    continue;
                } else {
                    eprintln!("argument expected for dir option\n");
                    usage(1);
                }
            }
            "-s" | "--size" => {
                opts.size = true;
                continue;
            }
            "-e" | "--errexit" => {
                opts.errexit = true;
                continue;
            }
            "--diff" => {
                opts.diff = true;
                continue;
            }
            "--" => {
                opts.cmd = Some(args.collect::<Vec<_>>().join(" "));
                break;
            }
            _ if arg.starts_with("-") => {}
            file => {
                opts.file = Some(file.into());
                break;
            }
        }

        // key=value options
        if let Some((key, val)) = arg.split_once("=") {
            match key {
                "--time" => {
                    if opts.time.is_some() {
                        eprintln!("time option specified multiple times\n");
                        usage(1);
                    }

                    if let Ok(t) = val.parse::<f64>() {
                        opts.time = Some(t);
                        continue;
                    } else {
                        eprintln!("numeric value expected for time argument\n");
                        usage(1);
                    }
                }
                "--file" => {
                    opts.files.push(val.to_string().into());
                    continue;
                }
                "--dir" => {
                    opts.files.push(val.to_string().into());
                    continue;
                }
                _ => {
                    eprintln!("unrecognized option: {arg}\n");
                    usage(1);
                }
            }
        }

        // special case for t cause i want -t2 to work
        if arg.starts_with("-t") {
            if opts.time.is_some() {
                eprintln!("time option specified multiple times\n");
                usage(1);
            }

            let (_, val) = arg.split_at(2);
            let t = val.parse::<f64>();
            if let Ok(t) = t {
                opts.time = Some(t);
            } else {
                eprintln!("numeric value expected for time argument\n");
                usage(1);
            }
        }
    }
    // check that the options are valid and that paths exist
    if opts.cmd.is_none() && opts.file.is_none() {
        eprintln!("must specify a file or a command\n");
        usage(1);
    }

    if opts.never
        && (opts.files.len() > 0 || opts.files.len() > 0 || opts.size || opts.time.is_some())
    {
        eprintln!("cannot specify never with other update options\n");
        usage(1);
    }

    // now that we checked "never" we can put the file in the watch list
    if let Some(f) = opts.file.as_ref() {
        if !opts.never {
            opts.files.push(f.clone());
        }
    }

    if let Some(f) = opts.file.as_ref() {
        if !f.exists() {
            eprintln!("file '{}' does not exist\n", f.to_string_lossy());
            usage(1);
        }
    }

    for f in opts.files.iter() {
        if !f.exists() {
            eprintln!("file '{}' does not exist\n", f.to_string_lossy());
            usage(1);
        }
    }

    opts
}
