use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use skillsdash::ui::{render, App, Controller};
use std::fs;
use std::path::PathBuf;

fn temp_project(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("skillsdash-render-{tag}"));
    let _ = fs::remove_dir_all(&base);
    let skills = base.join(".claude/skills/demo");
    fs::create_dir_all(&skills).unwrap();
    fs::write(
        skills.join("SKILL.md"),
        "---\nname: demo\ndescription: a demo skill for rendering\n---\n\n# Demo\n\nsome body text\n",
    )
    .unwrap();
    base
}

fn press(c: char) -> KeyEvent {
    key(KeyCode::Char(c))
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::NONE,
    }
}

fn type_str(app: &mut App, controller: &mut Controller, s: &str) {
    for c in s.chars() {
        controller.handle_key(app, press(c));
    }
}

fn draw(app: &App, controller: &Controller) {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render::render(f, app, controller))
        .unwrap();
}

#[test]
fn renders_every_screen_without_panic() {
    let dir = temp_project("screens");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    draw(&app, &controller);

    controller.handle_key(&mut app, press('a'));
    draw(&app, &controller);
    controller.handle_key(&mut app, press('x'));
    draw(&app, &controller);
    controller.handle_key(
        &mut app,
        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        },
    );

    controller.handle_key(&mut app, press('?'));
    draw(&app, &controller);
    controller.handle_key(
        &mut app,
        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        },
    );

    controller.handle_key(
        &mut app,
        KeyEvent {
            code: KeyCode::Enter,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        },
    );
    draw(&app, &controller);

    controller.handle_key(&mut app, press('e'));
    draw(&app, &controller);
    assert!(controller.editor.is_some(), "editor should be open");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn vim_navigation_moves_selection() {
    let dir = temp_project("vimnav");
    let claude = dir.join(".claude/skills");
    for name in ["alpha", "beta", "gamma"] {
        let d = claude.join(name);
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: d\n---\nbody\n"),
        )
        .unwrap();
    }
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();
    assert_eq!(app.selected, 0);
    controller.handle_key(&mut app, press('j'));
    assert_eq!(app.selected, 1);
    controller.handle_key(&mut app, press('j'));
    assert_eq!(app.selected, 2);
    controller.handle_key(&mut app, press('k'));
    assert_eq!(app.selected, 1);
    controller.handle_key(&mut app, press('G'));
    assert!(app.selected >= 2);
    controller.handle_key(&mut app, press('g'));
    controller.handle_key(&mut app, press('g'));
    assert_eq!(app.selected, 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn marketplace_opens_and_renders() {
    let dir = temp_project("market");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    controller.handle_key(&mut app, press('m'));
    assert!(controller.market.is_some());
    draw(&app, &controller);

    controller.handle_key(&mut app, press('r'));
    controller.handle_key(&mut app, press('e'));
    controller.handle_key(&mut app, press('a'));
    controller.handle_key(&mut app, press('c'));
    controller.handle_key(&mut app, press('t'));
    assert_eq!(controller.market.as_ref().unwrap().query, "react");
    draw(&app, &controller);

    controller.handle_key(
        &mut app,
        KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        },
    );
    draw(&app, &controller);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_flow_writes_skill_to_project_scope() {
    let dir = temp_project("createflow");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    controller.handle_key(&mut app, press('a'));
    type_str(&mut app, &mut controller, "brand-new");
    controller.handle_key(&mut app, key(KeyCode::Tab));
    type_str(&mut app, &mut controller, "a freshly made skill");
    controller.handle_key(&mut app, key(KeyCode::Tab));
    controller.handle_key(&mut app, key(KeyCode::Tab));
    controller.handle_key(&mut app, press(' '));
    controller.handle_key(&mut app, key(KeyCode::Enter));

    let expected = dir.join(".claude/skills/brand-new/SKILL.md");
    assert!(
        expected.exists(),
        "create flow should write {}",
        expected.display()
    );
    let content = fs::read_to_string(&expected).unwrap();
    assert!(content.contains("name: brand-new"));
    assert!(content.contains("a freshly made skill"));

    let skill = app
        .registry
        .skills
        .iter()
        .find(|s| s.name == "brand-new")
        .unwrap();
    assert!(skill.has(
        skillsdash::model::Provider::Claude,
        skillsdash::model::Scope::Project
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn editor_edits_and_saves_body() {
    let dir = temp_project("editflow");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    let idx = app
        .filtered_indices()
        .iter()
        .position(|&i| app.registry.skills[i].name == "demo")
        .unwrap();
    app.selected = idx;

    controller.handle_key(&mut app, press('e'));
    assert!(controller.editor.is_some());

    controller.handle_key(&mut app, press('G'));
    controller.handle_key(&mut app, press('o'));
    type_str(&mut app, &mut controller, "APPENDED LINE");
    controller.handle_key(&mut app, key(KeyCode::Esc));
    controller.handle_key(&mut app, press(':'));
    controller.handle_key(&mut app, press('w'));
    controller.handle_key(&mut app, key(KeyCode::Enter));

    let md = dir.join(".claude/skills/demo/SKILL.md");
    let content = fs::read_to_string(&md).unwrap();
    assert!(
        content.contains("APPENDED LINE"),
        "editor save should persist body edits:\n{content}"
    );
    assert!(
        content.contains("name: demo"),
        "frontmatter must be preserved"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn search_filters_list() {
    let dir = temp_project("search");
    let claude = dir.join(".claude/skills");
    for name in ["alpha", "beta", "gamma"] {
        let d = claude.join(name);
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: d\n---\nbody\n"),
        )
        .unwrap();
    }
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();
    let total = app.visible_count();
    assert!(total >= 3);

    controller.handle_key(&mut app, press('/'));
    controller.handle_key(&mut app, press('b'));
    controller.handle_key(&mut app, press('e'));
    controller.handle_key(&mut app, press('t'));
    controller.handle_key(&mut app, press('a'));

    let filtered = app.visible_count();
    assert!(filtered >= 1, "query 'beta' should match the beta fixture");
    assert!(filtered < total, "search should narrow the list");
    let names: Vec<String> = app
        .filtered_indices()
        .iter()
        .map(|&i| app.registry.skills[i].name.clone())
        .collect();
    assert!(
        names.iter().any(|n| n == "beta"),
        "beta must survive the filter, got {names:?}"
    );
    for name in &names {
        let s = app
            .registry
            .skills
            .iter()
            .find(|s| &s.name == name)
            .unwrap();
        let hay = format!("{} {}", s.name.to_lowercase(), s.description.to_lowercase());
        assert!(
            hay.contains("beta"),
            "every match must contain the query: {name}"
        );
    }

    let _ = fs::remove_dir_all(&dir);
}
