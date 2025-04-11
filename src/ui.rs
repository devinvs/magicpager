use std::io::{BufRead, Read, Write};
use std::{env::args, process::Command};

use crossterm::{cursor, style, terminal, ExecutableCommand, QueueableCommand};

struct State {
    cmd: Command,
    buf: Vec<String>,
    // position of the cursor with respect to the buffer
    buf_cur: (usize, usize),
    // position of the cursors with respect to the screen
    scr_cur: (usize, usize),
}

impl State {
    fn init(cmd: Command) -> State {
        std::io::stdout()
            .execute(terminal::EnterAlternateScreen)
            .unwrap();
        terminal::enable_raw_mode().unwrap();

        let mut me = State {
            cmd,
            buf: vec![],
            buf_cur: (0, 0),
            scr_cur: (0, 0),
        };

        me.update();
        me.draw();

        me
    }

    fn update(&mut self) {
        // an answer from the universe
        let res = self.cmd.output().unwrap();
        self.buf.clear();

        let mut lines = res.stdout.lines();
        while let Some(Ok(line)) = lines.next() {
            self.buf.push(line);
        }

        // a changing landscape, truth moves with its grounding
        self.buf_cur.1 = self.buf_cur.1.min(self.buf.len() - 1);
        self.buf_cur.0 = self.buf_cur.0.min(self.buf[self.buf_cur.1].len());
    }

    fn event(&mut self, c: char) {}

    fn draw(&mut self) {
        let (cols, rows) = terminal::size().unwrap();
        let mut stdout = std::io::stdout();

        let num_digs = self.buf.len().ilog10() + 1;

        // the buffer cursor, truth; the screen cursor, optimism.
        // ensure optimism falls within the window of truth
        self.scr_cur.1 = self.scr_cur.1.min(rows as usize);
        self.scr_cur.0 = self.scr_cur.0.min(cols as usize);

        self.scr_cur.1 = self.scr_cur.1 - self.scr_cur.1.saturating_sub(self.buf_cur.1);

        // the void, formless, empty. the perfect canvas
        stdout
            .queue(terminal::Clear(terminal::ClearType::All))
            .unwrap()
            .queue(terminal::DisableLineWrap)
            .unwrap()
            .queue(cursor::MoveTo(0, 0))
            .unwrap();
    }
}

impl Drop for State {
    fn drop(&mut self) {
        terminal::disable_raw_mode().unwrap();
        std::io::stdout()
            .execute(terminal::LeaveAlternateScreen)
            .unwrap();
    }
}

fn draw(path: &str, state: &mut State) {
    for (i, line) in term_lines.iter().enumerate() {
        let line_no = i + 1 + lines_read.saturating_sub(rows as usize);

        if i != 0 {
            stdout.queue(cursor::MoveDown(1)).unwrap();
        }

        stdout.queue(cursor::MoveToColumn(0)).unwrap();
        stdout
            .queue(style::SetForegroundColor(style::Color::DarkGrey))
            .unwrap();

        print!("{line_no:>width$}. ", width = num_digs as usize);
        stdout.queue(style::ResetColor).unwrap();
        print!("{line}");
    }

    stdout
        .queue(cursor::MoveTo(state.scol as u16 + 4, state.srow as u16))
        .unwrap();

    stdout.flush().unwrap();
}

fn main() {
    let path = args().nth(1).unwrap();

    draw(&path, &mut state);
    let mut cs = std::io::stdin().bytes();

    while let Some(Ok(c)) = cs.next() {
        match c {
            113 | 3 => {
                // q | ctrl-c
                break;
            }
            106 => {
                // j
                state.srow = state.srow.saturating_add(1);
                state.frow = state.frow.saturating_add(1);
            }
            107 => {
                // k
                state.srow = state.srow.saturating_sub(1);
                state.frow = state.frow.saturating_sub(1);
            }
            104 => {
                // h
                state.scol = state.scol.saturating_sub(1);
            }
            108 => {
                // l
                state.scol = state.scol.saturating_add(1);
            }
            _ => {}
        }

        draw(&path, &mut state);
    }
}
