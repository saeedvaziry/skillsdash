use anyhow::Result;
use crossterm::event::{self, Event};
use skillsdash::tui;
use skillsdash::ui::{render, App, Controller};
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "-V" | "--version" => {
                println!("skillsdash {VERSION}");
                return Ok(());
            }
            "-h" | "--help" => {
                print_help();
                return Ok(());
            }
            other => {
                eprintln!("skillsdash: unknown argument '{other}'");
                print_help();
                std::process::exit(2);
            }
        }
    }

    let project_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let mut app = App::new(project_dir);
    let mut controller = Controller::new();

    let mut terminal = tui::init()?;
    let _guard = tui::RestoreGuard;
    let result = run(&mut terminal, &mut app, &mut controller);
    tui::restore()?;
    result
}

fn print_help() {
    println!(
        "skillsdash {VERSION}
Cross-platform TUI for managing AI skills across Claude and Agents providers.

USAGE:
    skillsdash            launch the TUI in the current directory
    skillsdash --version  print version and exit
    skillsdash --help     print this help and exit

Skills are read from ~/.claude/skills, ~/.agents/skills, and the current
project's .claude/.agents directories. Press h for the harness view (CLAUDE.md /
AGENTS.md) or c for the commands view; both let you symlink one provider to
another. Press ? inside the app for keys."
    );
}

fn run(terminal: &mut tui::Tui, app: &mut App, controller: &mut Controller) -> Result<()> {
    loop {
        controller.tick(app);
        terminal.draw(|f| render::render(f, app, controller))?;

        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Key(key) => controller.handle_key(app, key),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
