use anyhow::Result;
use crossterm::event::{self, Event};
use skillsdash::tui;
use skillsdash::ui::{render, App, Controller};
use std::time::Duration;

fn main() -> Result<()> {
    let project_dir = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let mut app = App::new(project_dir);
    let mut controller = Controller::new();

    let mut terminal = tui::init()?;
    let _guard = tui::RestoreGuard;
    let result = run(&mut terminal, &mut app, &mut controller);
    tui::restore()?;
    result
}

fn run(
    terminal: &mut tui::Tui,
    app: &mut App,
    controller: &mut Controller,
) -> Result<()> {
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
