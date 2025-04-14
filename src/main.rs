use mp::ui;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();

    // parse the optional arguments

    // get command to run
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c");
    if args[1] != "--" {
        cmd.arg(format!("cat {}", args[1]));
    } else {
        cmd.arg(args[2..].join(" "));
    };

    let mut s = ui::State::init(cmd);
    s.runloop();
}
