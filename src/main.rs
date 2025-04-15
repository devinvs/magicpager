use std::io::prelude::*;
use std::sync::{Arc, Mutex};

use inotify::Inotify;
use mp::opts::parse_opts;
use mp::ui;

fn main() {
    let opts = parse_opts();

    // build the command
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("-c");
    if let Some(f) = opts.file.as_ref() {
        cmd.arg(format!("cat {}", f.to_string_lossy()));
    } else if let Some(c) = opts.cmd.as_ref() {
        cmd.arg(c);
    };

    // initialise the ui and surround with arc/mutex for sharing across threads
    let s = Arc::new(Mutex::new(ui::State::init(cmd, opts)));

    // for each watching operation, start a new thread
    if let Some(t) = s.lock().unwrap().opts.time {
        // small stack size cause it just loops
        let s = s.clone();
        std::thread::Builder::new()
            .stack_size(1024)
            .name("timer".to_string())
            .spawn(move || {
                timer_thread(s, t);
            })
            .unwrap();
    }

    if s.lock().unwrap().opts.files.len() > 0 {
        let s = s.clone();
        std::thread::spawn(move || {
            inotify_thread(s);
        });
    }

    ui_thread(s);
}

fn ui_thread(s: Arc<Mutex<ui::State>>) {
    let mut bytes = std::io::stdin().bytes();
    while let Some(Ok(c)) = bytes.next() {
        let mut s = s.lock().unwrap();

        s.event(c as char);
        s.draw();
    }

    s.lock().unwrap().exit();
}

fn timer_thread(s: Arc<Mutex<ui::State>>, t: f64) {
    loop {
        std::thread::sleep(std::time::Duration::from_secs_f64(t));
        let mut state = s.lock().unwrap();
        state.update();
        state.draw();
    }
}

fn inotify_thread(s: Arc<Mutex<ui::State>>) {
    let mut inotify = Inotify::init().unwrap();

    // add the files to be watched
    {
        let state = s.lock().unwrap();
        for f in state.opts.files.iter() {
            inotify
                .watches()
                .add(f, inotify::WatchMask::MODIFY)
                .unwrap();
        }
    }

    // wait for changes
    let mut buffer = [0; 1024];
    loop {
        inotify.read_events_blocking(&mut buffer).unwrap();
        let mut state = s.lock().unwrap();
        state.update();
        state.draw();
    }
}
