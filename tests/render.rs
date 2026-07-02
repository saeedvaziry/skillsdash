use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use skillsdash::model::{Provider, Scope};
use skillsdash::ui::app::Modal;
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

fn ctrl(c: char) -> KeyEvent {
    KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
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
fn harness_view_shows_only_memory_files_and_edits() {
    use skillsdash::model::HarnessKind;
    let dir = temp_project("harness");
    let home = dir.join("fakehome");
    fs::create_dir_all(&home).unwrap();
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    controller.handle_key(&mut app, press('h'));
    assert_eq!(app.screen, skillsdash::ui::app::Screen::Harness);
    let view = app.harness_view_files(HarnessKind::Memory);
    assert!(view.iter().all(|f| f.kind == HarnessKind::Memory));
    assert!(view.iter().any(|f| f.name == "CLAUDE.md"));
    assert!(view.iter().any(|f| f.name == "AGENTS.md"));
    draw(&app, &controller);

    controller.handle_key(&mut app, press('e'));
    assert!(controller.editor.is_some(), "memory editor should open");
    assert_eq!(app.screen, skillsdash::ui::app::Screen::Editor);
    draw(&app, &controller);

    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn commands_view_shows_only_commands_and_creates() {
    use skillsdash::model::HarnessKind;
    let dir = temp_project("commands");
    let home = dir.join("fakehome");
    fs::create_dir_all(&home).unwrap();
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    controller.handle_key(&mut app, press('c'));
    assert_eq!(app.screen, skillsdash::ui::app::Screen::Commands);
    assert!(app
        .harness_view_files(HarnessKind::Command)
        .iter()
        .all(|f| f.kind == HarnessKind::Command));
    draw(&app, &controller);

    controller.handle_key(&mut app, press('a'));
    type_str(&mut app, &mut controller, "review");
    controller.handle_key(&mut app, key(KeyCode::Enter));
    draw(&app, &controller);

    let cmds = app.harness_view_files(HarnessKind::Command);
    assert!(
        cmds.iter().any(|f| f.name == "review.md"),
        "created command should appear in the commands view"
    );
    assert!(!cmds.iter().any(|f| f.name == "CLAUDE.md"));

    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn harness_view_groups_by_scope_and_tab_switches() {
    use skillsdash::model::{HarnessKind, Scope};
    let dir = temp_project("harness-groups");
    let home = dir.join("fakehome");
    fs::create_dir_all(&home).unwrap();
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();
    controller.handle_key(&mut app, press('h'));

    let sections = app.harness_scope_sections(HarnessKind::Memory);
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].0, Scope::Global);
    assert_eq!(sections[1].0, Scope::Project);
    assert!(sections[0].1.iter().all(|(_, f)| f.scope == Scope::Global));
    assert!(sections[1].1.iter().all(|(_, f)| f.scope == Scope::Project));

    assert_eq!(
        app.harness_selected_file().map(|f| f.scope),
        Some(Scope::Global)
    );
    controller.handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(
        app.harness_selected_file().map(|f| f.scope),
        Some(Scope::Project)
    );
    controller.handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(
        app.harness_selected_file().map(|f| f.scope),
        Some(Scope::Global)
    );

    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn highlighted_none_row_keeps_foreground_off_the_highlight_background() {
    use ratatui::style::Color;
    let dir = temp_project("harness-none");
    let home = dir.join("fakehome");
    fs::create_dir_all(&home).unwrap();
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);

    fs::write(dir.join("CLAUDE.md"), "# project\n").unwrap();

    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();
    controller.handle_key(&mut app, press('h'));
    controller.handle_key(&mut app, key(KeyCode::Tab));
    controller.handle_key(&mut app, press('j'));

    let sel = app.harness_selected_file().unwrap();
    assert_eq!(sel.name, "AGENTS.md");
    assert!(!sel.exists, "AGENTS.md should be a (none) row");

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render::render(f, &app, &controller))
        .unwrap();
    let buf = terminal.backend().buffer().clone();

    let mut checked = 0;
    for y in 0..buf.area.height {
        let mut row = String::new();
        for x in 0..buf.area.width {
            row.push_str(buf.cell((x, y)).unwrap().symbol());
        }
        if row.contains("▌ AGENTS.md") && row.contains("(none)") {
            for x in 0..buf.area.width {
                let cell = buf.cell((x, y)).unwrap();
                if cell.symbol().trim().is_empty() {
                    continue;
                }
                if cell.bg == Color::Indexed(8) && cell.fg != Color::Reset {
                    assert_ne!(cell.fg, cell.bg, "text invisible against highlight bg");
                    checked += 1;
                }
            }
        }
    }
    assert!(
        checked > 0,
        "expected to inspect the highlighted (none) row"
    );

    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
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
    // Isolate HOME so the only skill is the project-scoped fixture; that keeps
    // the project box focused so `a` prefills scope = project.
    let home = dir.join("fakehome");
    fs::create_dir_all(&home).unwrap();
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    // Focus the project box, then create (providers default to claude,
    // scope prefilled to project). No scope toggle needed.
    app.focused_group = skillsdash::ui::app::SkillGroup::Project;
    controller.handle_key(&mut app, press('a'));
    type_str(&mut app, &mut controller, "brand-new");
    controller.handle_key(&mut app, key(KeyCode::Tab));
    type_str(&mut app, &mut controller, "a freshly made skill");
    controller.handle_key(&mut app, key(KeyCode::Enter));
    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }

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
fn create_flow_multi_select_providers_and_scopes() {
    use skillsdash::model::{Provider, Scope};

    let dir = temp_project("multi");
    // Isolate HOME so global writes land in the fixture, not the real home.
    let home = dir.join("fakehome");
    fs::create_dir_all(&home).unwrap();
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    controller.handle_key(&mut app, press('a'));
    type_str(&mut app, &mut controller, "everywhere");
    // Move to the provider field and check the second provider (agents) too.
    controller.handle_key(&mut app, key(KeyCode::Tab)); // -> description
    controller.handle_key(&mut app, key(KeyCode::Tab)); // -> provider
    controller.handle_key(&mut app, key(KeyCode::Right)); // cursor -> agents
    controller.handle_key(&mut app, press(' ')); // toggle agents on
                                                 // Move to scope field. One scope is pre-checked from the focused group;
                                                 // move the cursor to the other chip and check it too so both are on.
    controller.handle_key(&mut app, key(KeyCode::Tab)); // -> scope
    controller.handle_key(&mut app, key(KeyCode::Right)); // cursor -> other scope
    controller.handle_key(&mut app, press(' ')); // toggle it on
    controller.handle_key(&mut app, key(KeyCode::Enter));

    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }

    let skill = app
        .registry
        .skills
        .iter()
        .find(|s| s.name == "everywhere")
        .expect("skill created");
    // Created across both providers and both scopes = 4 instances.
    for provider in [Provider::Claude, Provider::Agents] {
        for scope in [Scope::Global, Scope::Project] {
            assert!(
                skill.has(provider, scope),
                "expected instance in {provider}/{scope}"
            );
        }
    }
    assert_eq!(skill.instances.len(), 4);

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
fn editor_ctrl_s_saves_and_returns_to_normal_mode() {
    use skillsdash::ui::editor::VimMode;
    let dir = temp_project("editctrls");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    let idx = app
        .filtered_indices()
        .iter()
        .position(|&i| app.registry.skills[i].name == "demo")
        .unwrap();
    app.selected = idx;

    controller.handle_key(&mut app, press('e'));
    controller.handle_key(&mut app, press('G'));
    controller.handle_key(&mut app, press('o'));
    type_str(&mut app, &mut controller, "CTRL S LINE");
    controller.handle_key(&mut app, ctrl('s'));

    let editor = controller
        .editor
        .as_ref()
        .expect("ctrl+s saves without closing the editor");
    assert_eq!(
        editor.mode,
        VimMode::Normal,
        "ctrl+s switches from insert to normal mode"
    );

    let md = dir.join(".claude/skills/demo/SKILL.md");
    let content = fs::read_to_string(&md).unwrap();
    assert!(
        content.contains("CTRL S LINE"),
        "ctrl+s should persist body edits from insert mode:\n{content}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn editor_q_quits_from_normal_mode_without_saving() {
    let dir = temp_project("editq");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    let idx = app
        .filtered_indices()
        .iter()
        .position(|&i| app.registry.skills[i].name == "demo")
        .unwrap();
    app.selected = idx;

    controller.handle_key(&mut app, press('e'));
    controller.handle_key(&mut app, press('G'));
    controller.handle_key(&mut app, press('o'));
    type_str(&mut app, &mut controller, "UNSAVED LINE");
    controller.handle_key(&mut app, key(KeyCode::Esc));
    controller.handle_key(&mut app, press('q'));

    assert!(
        controller.editor.is_none(),
        "q in normal mode closes the editor"
    );

    let md = dir.join(".claude/skills/demo/SKILL.md");
    let content = fs::read_to_string(&md).unwrap();
    assert!(
        !content.contains("UNSAVED LINE"),
        "q must not save the edit:\n{content}"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn grouping_separates_project_and_global_skills() {
    use skillsdash::ui::app::SkillGroup;

    let dir = temp_project("grouping");
    // temp_project already seeds a project-scoped "demo" skill under .claude/skills.
    // Add a global skill by pointing HOME at an isolated dir with its own skill.
    let home = dir.join("fakehome");
    let global = home.join(".claude/skills/globex");
    fs::create_dir_all(&global).unwrap();
    fs::write(
        global.join("SKILL.md"),
        "---\nname: globex\ndescription: a global skill\n---\nbody\n",
    )
    .unwrap();

    // Discover with HOME overridden so the global root resolves into our fixture.
    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let app = App::new(dir.clone());
    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }

    assert!(app.grouped, "grouping is on by default");

    // Two separate sections (boxes): global first (top), then project (bottom).
    let sections = app.grouped_sections();
    let groups: Vec<SkillGroup> = sections.iter().map(|(g, _)| *g).collect();
    assert_eq!(
        groups,
        vec![SkillGroup::Global, SkillGroup::Project],
        "global section comes before project section"
    );

    let name_in = |group: SkillGroup, name: &str| {
        sections
            .iter()
            .find(|(g, _)| *g == group)
            .map(|(_, rows)| {
                rows.iter()
                    .any(|&(_, ri)| app.registry.skills[ri].name == name)
            })
            .unwrap_or(false)
    };
    assert!(
        name_in(SkillGroup::Project, "demo"),
        "demo is a project skill"
    );
    assert!(
        name_in(SkillGroup::Global, "globex"),
        "globex is a global skill"
    );

    // Renders two stacked boxes without panicking.
    let controller = Controller::new();
    draw(&app, &controller);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn tab_swaps_focus_between_project_and_global_boxes() {
    use skillsdash::ui::app::SkillGroup;

    let dir = temp_project("tab-focus");
    // temp_project seeds a project-scoped "demo". Add a global skill via HOME.
    let home = dir.join("fakehome");
    let global = home.join(".claude/skills/globex");
    fs::create_dir_all(&global).unwrap();
    fs::write(
        global.join("SKILL.md"),
        "---\nname: globex\ndescription: a global skill\n---\nbody\n",
    )
    .unwrap();

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let mut app = App::new(dir.clone());
    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
    let mut controller = Controller::new();

    // Selection starts in the global box (global section renders first/top).
    assert_eq!(app.focused_group, SkillGroup::Global);
    assert_eq!(app.selected_group(), Some(SkillGroup::Global));

    // Tab jumps the selection into the project box.
    controller.handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(app.focused_group, SkillGroup::Project);
    assert_eq!(app.selected_skill().map(|s| s.name.as_str()), Some("demo"));

    // Tab again jumps back to the global box.
    controller.handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(app.focused_group, SkillGroup::Global);
    assert_eq!(
        app.selected_skill().map(|s| s.name.as_str()),
        Some("globex")
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn both_group_boxes_render_when_one_is_empty() {
    use skillsdash::ui::app::SkillGroup;

    // A project with skills but NO global skills (HOME points at an empty dir).
    let dir = temp_project("empty-global");
    let home = dir.join("emptyhome");
    fs::create_dir_all(&home).unwrap();

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let app = App::new(dir.clone());
    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }

    // Both group sections exist; global is present but empty.
    let sections = app.grouped_sections();
    let groups: Vec<SkillGroup> = sections.iter().map(|(g, _)| *g).collect();
    assert_eq!(groups, vec![SkillGroup::Global, SkillGroup::Project]);
    let global = sections
        .iter()
        .find(|(g, _)| *g == SkillGroup::Global)
        .unwrap();
    assert!(global.1.is_empty(), "no global skills in this fixture");

    // Renders both boxes (empty global box included) without panicking.
    let controller = Controller::new();
    draw(&app, &controller);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn create_form_prefills_scope_from_focused_group() {
    use skillsdash::model::Scope;
    use skillsdash::ui::app::{FormKind, SkillGroup};

    // Project + global skills so both boxes are populated and Tab can move focus.
    let dir = temp_project("prefill");
    let home = dir.join("fakehome");
    let global = home.join(".claude/skills/globex");
    fs::create_dir_all(&global).unwrap();
    fs::write(
        global.join("SKILL.md"),
        "---\nname: globex\ndescription: a global skill\n---\nbody\n",
    )
    .unwrap();

    let prev_home = std::env::var_os("HOME");
    std::env::set_var("HOME", &home);
    let mut app = App::new(dir.clone());
    match prev_home {
        Some(h) => std::env::set_var("HOME", h),
        None => std::env::remove_var("HOME"),
    }
    let mut controller = Controller::new();

    // Startup focus is the global box (global section renders first/top) ->
    // `a` prefills the scope selection to global only.
    assert_eq!(app.focused_group, SkillGroup::Global);
    controller.handle_key(&mut app, press('a'));
    let form = app.form.as_ref().expect("create form open");
    assert_eq!(form.kind, FormKind::Create);
    assert_eq!(form.selected_scopes(), vec![Scope::Global]);
    controller.handle_key(&mut app, key(KeyCode::Esc));

    // Focus the project box, then `a` prefills scope = project only.
    controller.handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(app.focused_group, SkillGroup::Project);
    controller.handle_key(&mut app, press('a'));
    let form = app.form.as_ref().expect("create form open");
    assert_eq!(form.selected_scopes(), vec![Scope::Project]);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn toggle_grouping_collapses_to_single_section() {
    let dir = temp_project("toggle-group");
    let mut app = App::new(dir.clone());
    let mut controller = Controller::new();

    assert!(app.grouped, "grouping is on by default");

    controller.handle_key(&mut app, press('o'));
    assert!(!app.grouped, "'o' toggles grouping off");
    assert_eq!(
        app.grouped_sections().len(),
        1,
        "ungrouped view is a single flat section"
    );
    draw(&app, &controller);

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

fn buffer_text(app: &App, controller: &Controller) -> String {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| render::render(f, app, controller))
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf.cell((x, y)).unwrap().symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn install_modal_multi_select_toggles_and_renders_checkboxes() {
    let dir = temp_project("install-multi");
    let mut app = App::new(dir.clone());
    let controller = Controller::new();

    let options = vec![
        (Provider::Claude, Scope::Global),
        (Provider::Claude, Scope::Project),
        (Provider::Agents, Scope::Global),
    ];
    app.modal = Modal::InstallTarget {
        skill_name: "demo".to_string(),
        options: options.clone(),
        checked: vec![false; options.len()],
        cursor: 0,
    };

    let screen = buffer_text(&app, &controller);
    assert!(screen.contains("[ ]"), "unchecked boxes render");
    assert!(
        screen.contains("space select"),
        "hint mentions space-to-select"
    );

    let mut controller = controller;
    controller.handle_key(&mut app, press(' '));
    controller.handle_key(&mut app, press('j'));
    controller.handle_key(&mut app, press('j'));
    controller.handle_key(&mut app, press(' '));

    match &app.modal {
        Modal::InstallTarget { checked, .. } => {
            assert_eq!(
                checked,
                &vec![true, false, true],
                "space toggles the row under the cursor"
            );
        }
        _ => panic!("install modal should stay open after toggling"),
    }

    let screen = buffer_text(&app, &controller);
    assert!(screen.contains("[✓]"), "checked boxes render after toggle");

    let _ = fs::remove_dir_all(&dir);
}
