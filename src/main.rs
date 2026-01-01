use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent, read},
    execute,
    style::{Color, Print, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode, size},
};
use meval::{Context, eval_str, eval_str_with_context};
use std::collections::VecDeque;
use std::io::{self, Write, stdout};

#[derive(Debug, PartialEq, Clone, Copy)]
enum Mode {
    Normal,
    Insert,
    Command,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum WorkMode {
    Default,
    Multi,
}

struct App {
    lines: Vec<String>,
    current_line: usize,
    cursor_idx: usize,
    mode: Mode,
    work_mode: WorkMode,
    show_help: Option<String>,
}

fn format_res(val: f64) -> String {
    if (val - val.round()).abs() < 1e-10 {
        format!("{}", val.round() as i64)
    } else {
        format!("{:.4}", val)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn clean_input(input: &str) -> String {
    let mut s = input.replace("root", "sqrt").replace(" ", "");
    if s.contains('%') {
        if let Some(pos) = s.find('%') {
            if let Some(op_p) = s[..pos].rfind(|c| c == '+' || c == '-' || c == '*' || c == '/') {
                let (base, perc, op) = (&s[..op_p], &s[op_p + 1..pos], &s[op_p..op_p + 1]);
                return format!("{} {} ({} * {} / 100)", base, op, base, perc);
            }
        }
    }
    s
}

fn solve_logic(expr: &str, target: f64, var: &str) -> Option<f64> {
    let (mut l, mut h) = (-1e6, 1e6);
    let f = |v: f64| {
        let mut c = Context::new();
        c.var(var, v);
        eval_str_with_context(expr, &c).unwrap_or(f64::NAN)
    };
    for _ in 0..100 {
        let m = (l + h) / 2.0;
        if (f(l) - target) * (f(m) - target) <= 0.0 {
            h = m;
        } else {
            l = m;
        }
    }
    let r = (l + h) / 2.0;
    if (f(r) - target).abs() < 0.01 {
        Some(r)
    } else {
        None
    }
}

fn draw_help(stdout: &mut io::Stdout, cols: u16, rows: u16, lang: &str) -> io::Result<()> {
    let bw = 64;
    let bh = 16;
    let (sx, sy) = ((cols - bw) / 2, (rows - bh) / 2);
    let bg = Color::Black;

    let en = [
        (" --- DOCUMENTATION --- ", Color::Yellow),
        (" BASIC:  +, -, *, /, %, ^, pi, e ", Color::White),
        (" ROOTS:  sqrt(x), cbrt(x)        ", Color::Cyan),
        (" FUNCS:  abs, exp, ln, log10     ", Color::Cyan),
        (" ROUND:  floor, ceil, round      ", Color::Cyan),
        (" TRIG:   sin, cos, tan, asin     ", Color::Cyan),
        (" HYPER:  sinh, cosh, tanh        ", Color::Cyan),
        (" ------------------------------- ", Color::DarkGrey),
        (" ALGEBRA: a + 5 = 15             ", Color::Green),
        (" RESULT:  a = (shows 10)         ", Color::Green),
        (" ------------------------------- ", Color::DarkGrey),
        (" :d - Default | :m - Multi       ", Color::Yellow),
        (" :c - Clear   | :q - Quit        ", Color::Yellow),
        (" (Press ESC to close)            ", Color::White),
    ];

    let ru = [
        (" --- ДОКУМЕНТАЦИЯ --- ", Color::Yellow),
        (" БАЗА:   +, -, *, /, %, ^, pi, e ", Color::White),
        (" КОРНИ:  sqrt(x), cbrt(x)        ", Color::Cyan),
        (" ФУНКЦ:  abs, exp, ln, log10     ", Color::Cyan),
        (" ОКРУГ:  floor, ceil, round      ", Color::Cyan),
        (" ТРИГ:   sin, cos, tan, asin     ", Color::Cyan),
        (" ГИПЕР:  sinh, cosh, tanh        ", Color::Cyan),
        (" ------------------------------- ", Color::DarkGrey),
        (" АЛГЕБРА: x + 10 = 50            ", Color::Green),
        (" ОТВЕТ:   x = (выдаст 40)        ", Color::Green),
        (" ------------------------------- ", Color::DarkGrey),
        (" :d - Обычный | :m - Мульти      ", Color::Yellow),
        (" :c - Очистить | :q - Выход      ", Color::Yellow),
        (" (Нажми ESC для выхода)          ", Color::White),
    ];

    let content = if lang == "ru" { &ru } else { &en };

    for (i, (line, text_color)) in content.iter().enumerate() {
        execute!(
            stdout,
            cursor::MoveTo(sx, sy + i as u16),
            SetBackgroundColor(bg),
            SetForegroundColor(*text_color),
            Print(format!("  {:^width$}  ", line, width = (bw - 4) as usize))
        )?;
    }
    execute!(
        stdout,
        SetBackgroundColor(Color::Reset),
        SetForegroundColor(Color::Reset)
    )?;
    Ok(())
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    let mut app = App {
        lines: vec![String::new()],
        current_line: 0,
        cursor_idx: 0,
        mode: Mode::Normal,
        work_mode: WorkMode::Default,
        show_help: None,
    };
    let mut cmd_buf = String::new();
    let mut logs: VecDeque<String> = VecDeque::from(vec!["-".into(), "-".into(), "-".into()]);

    loop {
        let (cols, rows) = size()?;
        execute!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
        let mut ctx = Context::new();

        if app.work_mode == WorkMode::Multi {
            for line in &app.lines {
                if line.contains('=') {
                    let p: Vec<&str> = line.split('=').collect();
                    if p.len() == 2 {
                        let (lhs, rhs) = (clean_input(p[0]), clean_input(p[1]));
                        if let Ok(rv) = eval_str_with_context(&rhs, &ctx) {
                            if lhs.chars().all(|c| c.is_alphabetic()) {
                                ctx.var(lhs, rv);
                            } else if let Some(pos) = lhs.find(|c: char| c.is_alphabetic()) {
                                let vn = &lhs[pos..pos + 1];
                                if let Some(s) = solve_logic(&lhs, rv, vn) {
                                    ctx.var(vn, s);
                                }
                            }
                        }
                    }
                }
            }
            for (i, line) in app.lines.iter().enumerate() {
                execute!(
                    stdout,
                    cursor::MoveTo(0, i as u16),
                    SetForegroundColor(Color::DarkGrey)
                )?;
                print!("{:2} ", i + 1);
                execute!(stdout, SetForegroundColor(Color::White))?;
                print!("{}", line);
                let is_q =
                    line.contains('=') && line.split('=').last().unwrap_or("").trim().is_empty();
                if (is_q || !line.contains('=')) && !line.trim().is_empty() {
                    let target = clean_input(line.split('=').next().unwrap_or(""));
                    if let Ok(v) = eval_str_with_context(&target, &ctx) {
                        execute!(stdout, SetForegroundColor(Color::Green))?;
                        print!(" // {}", format_res(v));
                    }
                }
            }
        } else {
            let line = clean_input(&app.lines[0]);
            execute!(stdout, SetForegroundColor(Color::White))?;
            print!("❯ {}", app.lines[0]);
            if let Ok(res) = eval_str(&line) {
                execute!(stdout, SetForegroundColor(Color::Green))?;
                print!(" = {}", format_res(res));
            }
        }

        if let Some(ref lang) = app.show_help {
            draw_help(&mut stdout, cols, rows, lang)?;
        }

        execute!(
            stdout,
            cursor::MoveTo(0, rows - 3),
            SetForegroundColor(Color::DarkGrey)
        )?;
        print!("{}", "━".repeat(cols as usize));
        execute!(stdout, cursor::MoveTo(0, rows - 2))?;
        let w_c = if app.work_mode == WorkMode::Default {
            Color::Cyan
        } else {
            Color::Magenta
        };
        execute!(
            stdout,
            SetBackgroundColor(w_c),
            SetForegroundColor(Color::Black)
        )?;
        print!(" {:?} ", app.work_mode);
        let (m_t, m_b) = match app.mode {
            Mode::Normal => (" NORMAL ", Color::Blue),
            Mode::Insert => (" INSERT ", Color::Green),
            Mode::Command => (" COMMAND ", Color::Yellow),
        };
        execute!(stdout, SetBackgroundColor(m_b))?;
        print!("{}", m_t);
        execute!(
            stdout,
            SetBackgroundColor(Color::DarkGrey),
            SetForegroundColor(Color::White)
        )?;
        print!(" {} {} {} ", logs[0], logs[1], logs[2]);
        execute!(stdout, SetBackgroundColor(Color::Reset))?;
        print!(
            " pos: {}/{}",
            app.cursor_idx,
            app.lines[app.current_line].len()
        );

        execute!(
            stdout,
            cursor::MoveTo(0, rows - 1),
            Clear(ClearType::CurrentLine)
        )?;
        if app.mode == Mode::Command {
            execute!(stdout, SetForegroundColor(Color::Yellow))?;
            print!(":{}", cmd_buf);
        }

        if app.mode == Mode::Command {
            execute!(stdout, cursor::MoveTo((1 + cmd_buf.len()) as u16, rows - 1))?;
        } else {
            let x = if app.work_mode == WorkMode::Default {
                2
            } else {
                3
            };
            execute!(
                stdout,
                cursor::MoveTo((x + app.cursor_idx) as u16, app.current_line as u16)
            )?;
        }
        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = read()? {
            match app.mode {
                Mode::Normal => match code {
                    KeyCode::Esc => app.show_help = None,
                    KeyCode::Char('i') => app.mode = Mode::Insert,
                    KeyCode::Char(':') => {
                        app.mode = Mode::Command;
                        cmd_buf.clear();
                    }
                    KeyCode::Char('j') | KeyCode::Down if app.work_mode == WorkMode::Multi => {
                        if app.current_line < app.lines.len() - 1 {
                            app.current_line += 1;
                            app.cursor_idx = app.cursor_idx.min(app.lines[app.current_line].len());
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up if app.work_mode == WorkMode::Multi => {
                        if app.current_line > 0 {
                            app.current_line -= 1;
                            app.cursor_idx = app.cursor_idx.min(app.lines[app.current_line].len());
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        if app.cursor_idx > 0 {
                            app.cursor_idx -= 1
                        }
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        if app.cursor_idx < app.lines[app.current_line].len() {
                            app.cursor_idx += 1
                        }
                    }
                    _ => {}
                },
                Mode::Insert => match code {
                    KeyCode::Esc => app.mode = Mode::Normal,
                    KeyCode::Char(c) => {
                        if c == 'k' && app.lines[app.current_line].ends_with('j') {
                            app.lines[app.current_line].pop();
                            app.cursor_idx -= 1;
                            app.mode = Mode::Normal;
                        } else {
                            app.lines[app.current_line].insert(app.cursor_idx, c);
                            app.cursor_idx += 1;
                        }
                    }
                    KeyCode::Enter if app.work_mode == WorkMode::Multi => {
                        app.lines.insert(app.current_line + 1, String::new());
                        app.current_line += 1;
                        app.cursor_idx = 0;
                    }
                    KeyCode::Backspace => {
                        if app.cursor_idx > 0 {
                            app.lines[app.current_line].remove(app.cursor_idx - 1);
                            app.cursor_idx -= 1;
                        }
                    }
                    _ => {}
                },
                Mode::Command => match code {
                    KeyCode::Enter => {
                        match cmd_buf.as_str() {
                            "help" => app.show_help = Some("en".into()),
                            "help_ru" => app.show_help = Some("ru".into()),
                            "q" => break,
                            "d" => {
                                app.work_mode = WorkMode::Default;
                                app.lines = vec![app.lines[app.current_line].clone()];
                                app.current_line = 0;
                            }
                            "m" => app.work_mode = WorkMode::Multi,
                            "c" => {
                                app.lines = vec![String::new()];
                                app.current_line = 0;
                                app.cursor_idx = 0;
                            }
                            _ => {}
                        }
                        cmd_buf.clear();
                        app.mode = Mode::Normal;
                    }
                    KeyCode::Char(c) => cmd_buf.push(c),
                    KeyCode::Backspace => {
                        cmd_buf.pop();
                    }
                    KeyCode::Esc => app.mode = Mode::Normal,
                    _ => {}
                },
            }
        }
    }
    disable_raw_mode()?;
    Ok(())
}
