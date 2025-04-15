extern crate unicode_segmentation;

use std::io::{BufRead, Write};
use std::process::Command;

use crossterm::{cursor, style, terminal, ExecutableCommand, QueueableCommand};
use unicode_segmentation::UnicodeSegmentation;

use crate::opts::Options;

#[derive(PartialEq, Eq)]
enum Action {
    Exit,
    Up(usize),
    Down(usize),
    Left(usize),
    Right(usize),
    Jump(usize, usize),
    Cont,
}

enum Mode {
    Normal,
    Esc,
    Csi(String),
    Goto(String),
}

impl Mode {
    fn to_string(&self) -> String {
        match self {
            Mode::Normal => "    ".into(),
            Mode::Esc => "ESC ".into(),
            Mode::Csi(s) => format!("CSI {s}"),
            Mode::Goto(s) => format!("g{s:3}"),
        }
    }
}

pub struct State {
    pub cmd: Command,
    pub buf: Vec<String>,

    // position of the cursor in the terminal window
    pub cursor: (u16, u16),
    // size of the terminal
    pub term_size: (u16, u16),
    // how much weve scrolled through the buffer
    pub scroll: (usize, usize),
    mode: Mode,
    pub opts: Options,
}

impl State {
    pub fn init(cmd: Command, opts: Options) -> State {
        std::io::stdout()
            .execute(terminal::EnterAlternateScreen)
            .unwrap();
        terminal::enable_raw_mode().unwrap();

        let mut me = State {
            cmd,
            buf: vec![],
            cursor: (0, 0),
            term_size: (0, 0),
            scroll: (0, 0),
            mode: Mode::Normal,
            opts,
        };

        me.update();
        me.draw();

        me
    }

    pub fn update(&mut self) {
        // execute command, parse lines, store in buffer
        let res = self.cmd.output().unwrap();
        self.buf.clear();

        let mut lines = res.stdout.lines();
        while let Some(Ok(line)) = lines.next() {
            self.buf.push(line);
        }

        // ensure that the scroll position/cursor is within the text
        while self.scroll.1 + self.cursor.1 as usize > self.buf.len().saturating_sub(1) {
            self.up();
        }
    }

    fn left(&mut self) {
        if self.cursor.0 == 0 && self.scroll.0 > 0 {
            self.scroll.0 = self.scroll.0.saturating_sub(1);
        } else {
            self.cursor.0 = self.cursor.0.saturating_sub(1);
        }
    }

    fn right(&mut self) {
        if self.cursor.0 == self.term_size.0 - 1 {
            self.scroll.0 += 1;
        } else {
            self.cursor.0 = (self.cursor.0 + 1).min(self.term_size.0 - 1);
        }
    }

    fn up(&mut self) {
        let vscroll = self.term_size.1 / 5;
        if self.cursor.1 <= vscroll && self.scroll.1 > 0 {
            self.scroll.1 = self.scroll.1.saturating_sub(1);
        } else {
            self.cursor.1 = self.cursor.1.saturating_sub(1);
        }
    }

    fn down(&mut self) {
        let vscroll = self.term_size.1 / 5;
        if self.cursor.1 >= vscroll * 4
            && self.scroll.1 + (self.term_size.1 as usize - 1) < self.buf.len()
        {
            self.scroll.1 += 1;
        } else {
            self.cursor.1 = (self.cursor.1 + 1).min(self.term_size.1 - 2);
        }
    }

    pub fn event(&mut self, c: char) {
        let action = match &mut self.mode {
            Mode::Normal => match c {
                'q' | '\x03' => Action::Exit,
                'j' => Action::Down(1),
                'k' => Action::Up(1),
                'h' => Action::Left(1),
                'l' => Action::Right(1),
                '\x1b' => {
                    self.mode = Mode::Esc;
                    Action::Cont
                }
                'g' => {
                    self.mode = Mode::Goto(String::new());
                    Action::Cont
                }
                _ => Action::Cont,
            },
            Mode::Esc => match c {
                '[' => {
                    self.mode = Mode::Csi(String::new());
                    Action::Cont
                }
                _ => {
                    self.mode = Mode::Normal;
                    Action::Cont
                }
            },
            Mode::Csi(num) => match c {
                _ if c.is_ascii_digit() => {
                    num.push(c);
                    Action::Cont
                }
                'a' => {
                    if num.len() > 0 {
                        Action::Up(num.parse().unwrap())
                    } else {
                        Action::Up(1)
                    }
                }
                'b' => {
                    if num.len() > 0 {
                        Action::Down(num.parse().unwrap())
                    } else {
                        Action::Down(1)
                    }
                }
                'c' => {
                    if num.len() > 0 {
                        Action::Right(num.parse().unwrap())
                    } else {
                        Action::Right(1)
                    }
                }
                'd' => {
                    if num.len() > 0 {
                        Action::Left(num.parse().unwrap())
                    } else {
                        Action::Left(1)
                    }
                }
                '~' => {
                    if num == "5" {
                        Action::Up(self.term_size.1 as usize - 1)
                    } else if num == "6" {
                        Action::Down(self.term_size.1 as usize - 1)
                    } else {
                        Action::Cont
                    }
                }
                _ => {
                    self.mode = Mode::Normal;
                    Action::Cont
                }
            },
            Mode::Goto(num) => match c {
                _ if c.is_ascii_digit() => {
                    num.push(c);
                    Action::Cont
                }
                'g' => {
                    let n = if num.len() == 0 {
                        0_usize
                    } else {
                        num.parse().unwrap()
                    }
                    .saturating_sub(1);

                    Action::Jump(n, 0)
                }
                'e' => Action::Jump(self.buf.len(), 0),
                'h' => Action::Jump(self.cursor.1 as usize + self.scroll.1, 0),
                'l' => {
                    let i = self.cursor.1 as usize + self.scroll.1;
                    let line = &self.buf[i];
                    Action::Jump(i, line.len())
                }
                's' => {
                    let i = self.cursor.1 as usize + self.scroll.1;
                    let n = self.buf[i]
                        .chars()
                        .enumerate()
                        .skip_while(|(_, c)| c.is_whitespace())
                        .next()
                        .unwrap_or((0, '?'))
                        .0;
                    Action::Jump(i, n)
                }
                _ => {
                    self.mode = Mode::Normal;
                    Action::Cont
                }
            },
        };

        match action {
            // exit the pager
            Action::Exit => self.exit(),
            // scrolling/moving the cursor.
            //   we use a movement window which doesn't scroll
            Action::Down(n) => {
                for _ in 0..n {
                    self.down()
                }
            }
            Action::Up(n) => {
                for _ in 0..n {
                    self.up()
                }
            }
            Action::Left(n) => {
                for _ in 0..n {
                    self.left()
                }
            }
            Action::Right(n) => {
                for _ in 0..n {
                    self.right()
                }
            }
            Action::Jump(row, col) => {
                let row = row.min(self.buf.len() - 1);

                while self.scroll.1 + (self.cursor.1 as usize) < row {
                    self.down();
                }

                while self.scroll.1 + (self.cursor.1 as usize) > row {
                    self.up();
                }

                while self.scroll.0 + (self.cursor.0 as usize) < col {
                    self.right();
                }

                while self.scroll.0 + (self.cursor.0 as usize) > col {
                    self.left();
                }
            }
            Action::Cont => {}
        }

        if action != Action::Cont {
            self.mode = Mode::Normal;
        }
    }

    pub fn draw(&mut self) {
        self.term_size = terminal::size().unwrap();
        let mut stdout = std::io::stdout();

        let num_digs = if self.buf.len() == 0 {
            0
        } else {
            self.buf.len().ilog10()
        } + 1;

        let start_row = self.scroll.1;
        let end_row = (start_row + self.term_size.1 as usize - 1).min(self.buf.len());

        stdout
            .queue(terminal::Clear(terminal::ClearType::All))
            .unwrap()
            .queue(terminal::DisableLineWrap)
            .unwrap()
            .queue(cursor::MoveTo(0, 0))
            .unwrap();

        for (i, line) in self.buf[start_row..end_row].iter().enumerate() {
            if i != 0 {
                stdout.queue(cursor::MoveDown(1)).unwrap();
            }

            stdout.queue(cursor::MoveToColumn(0)).unwrap();
            stdout
                .queue(style::SetForegroundColor(style::Color::DarkGrey))
                .unwrap();

            print!("{:>width$}│ ", start_row + i + 1, width = num_digs as usize);
            stdout.queue(style::ResetColor).unwrap();

            if self.scroll.0 > 0 {
                UnicodeSegmentation::graphemes(line.as_str(), true)
                    .skip(self.scroll.0)
                    .for_each(|s| print!("{s}"));
            } else {
                print!("{line}");
            }
        }

        // draw bottom bar
        stdout
            .queue(cursor::MoveTo(0, self.term_size.1 - 1))
            .unwrap()
            .queue(style::SetForegroundColor(style::Color::Magenta))
            .unwrap();

        print!(
            "{} {}",
            self.mode.to_string(),
            if let Some(f) = self.opts.file.as_ref() {
                f.to_string_lossy()
            } else {
                self.cmd
                    .get_args()
                    .map(|arg| arg.to_string_lossy())
                    .nth(1)
                    .unwrap()
            }
        );

        let cur_s = format!(
            "{}:{}",
            self.cursor.1 as usize + self.scroll.1 + 1,
            self.cursor.0 as usize + self.scroll.0 + 1
        );
        stdout
            .queue(cursor::MoveToColumn(self.term_size.0 - cur_s.len() as u16))
            .unwrap();
        print!("{}", cur_s);

        stdout
            .queue(cursor::MoveTo(
                num_digs as u16 + 2 + self.cursor.0 as u16,
                self.cursor.1 as u16,
            ))
            .unwrap();
        stdout.flush().unwrap();
    }

    pub fn exit(&mut self) -> ! {
        terminal::disable_raw_mode().unwrap();
        std::io::stdout()
            .execute(terminal::LeaveAlternateScreen)
            .unwrap();

        std::process::exit(0);
    }
}
