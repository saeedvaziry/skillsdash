use super::actions::{self, ShareMethod};
use super::app::{App, FormField, FormKind, FormState, Modal, Screen};
use super::editor::{Editor, EditorSignal};
use super::market::{JobEvent, Market, MarketFocus};
use crate::model::{Provider, Scope};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub struct Controller {
    pub editor: Option<Editor>,
    pub market: Option<Market>,
}

impl Default for Controller {
    fn default() -> Self {
        Controller::new()
    }
}

impl Controller {
    pub fn new() -> Controller {
        Controller {
            editor: None,
            market: None,
        }
    }

    pub fn tick(&mut self, app: &mut App) {
        let events = if let Some(market) = self.market.as_mut() {
            market.tick();
            market.poll()
        } else {
            Vec::new()
        };
        for ev in events {
            match ev {
                JobEvent::Info(msg) => app.set_status(msg, false),
                JobEvent::Error(msg) => app.open_message("marketplace error", msg, true),
            }
        }
    }

    pub fn handle_key(&mut self, app: &mut App, key: KeyEvent) {
        if key.kind != crossterm::event::KeyEventKind::Press {
            return;
        }
        app.clear_status();

        if app.modal != Modal::None {
            self.handle_modal(app, key);
            return;
        }

        match app.screen {
            Screen::List => self.handle_list(app, key),
            Screen::Detail => self.handle_detail(app, key),
            Screen::Editor => self.handle_editor(app, key),
            Screen::Form => self.handle_form(app, key),
            Screen::Help => self.handle_help(app, key),
            Screen::Marketplace => self.handle_marketplace(app, key),
        }
    }

    fn handle_list(&mut self, app: &mut App, key: KeyEvent) {
        if app.search_active {
            self.handle_search_input(app, key);
            return;
        }

        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        if app.pending_g {
            app.pending_g = false;
            if key.code == KeyCode::Char('g') {
                app.select_first();
                return;
            }
        }

        match key.code {
            KeyCode::Char('d') if ctrl => app.move_selection(10),
            KeyCode::Char('u') if ctrl => app.move_selection(-10),
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => app.move_selection(1),
            KeyCode::Char('k') | KeyCode::Up => app.move_selection(-1),
            KeyCode::Char('g') => app.pending_g = true,
            KeyCode::Char('G') => app.select_last(),
            KeyCode::Char('/') => {
                app.search_active = true;
                app.search = Some(String::new());
            }
            KeyCode::Char('n') => self.jump_search(app, 1),
            KeyCode::Char('N') => self.jump_search(app, -1),
            KeyCode::Esc => {
                if app.search.is_some() {
                    app.search = None;
                    app.clamp_selection();
                }
            }
            KeyCode::Tab | KeyCode::BackTab => app.focus_other_group(),
            KeyCode::Char('t') => {
                app.scope_filter = app.scope_filter.next();
                app.clamp_selection();
            }
            KeyCode::Char('o') => {
                let name = app.selected_skill().map(|s| s.name.clone());
                app.grouped = !app.grouped;
                if let Some(name) = name {
                    if let Some(pos) = app
                        .filtered_indices()
                        .iter()
                        .position(|&i| app.registry.skills[i].name == name)
                    {
                        app.selected = pos;
                    }
                }
                app.clamp_selection();
                app.set_status(
                    if app.grouped {
                        "grouped by scope"
                    } else {
                        "grouping off"
                    },
                    false,
                );
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                if app.selected_skill().is_some() {
                    app.detail_scroll = 0;
                    app.prev_screen = Screen::List;
                    app.screen = Screen::Detail;
                }
            }
            KeyCode::Char('a') => self.open_create_form(app),
            KeyCode::Char('e') => self.open_editor(app),
            KeyCode::Char('f') => self.open_frontmatter_form(app),
            KeyCode::Char('s') => self.open_share(app),
            KeyCode::Char('m') | KeyCode::Char('M') => self.open_marketplace(app),
            KeyCode::Char('x') | KeyCode::Char('D') => self.open_delete(app),
            KeyCode::Char('r') => {
                app.reload();
                app.set_status("reloaded", false);
            }
            KeyCode::Char('?') => {
                app.prev_screen = Screen::List;
                app.screen = Screen::Help;
            }
            _ => {}
        }
    }

    fn handle_search_input(&mut self, app: &mut App, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                app.search_active = false;
                app.search = None;
                app.clamp_selection();
            }
            KeyCode::Enter => {
                app.search_active = false;
                if let Some(q) = &app.search {
                    app.last_search = q.clone();
                }
                app.clamp_selection();
            }
            KeyCode::Backspace => {
                if let Some(q) = app.search.as_mut() {
                    q.pop();
                }
                app.selected = 0;
            }
            KeyCode::Char(c) => {
                if let Some(q) = app.search.as_mut() {
                    q.push(c);
                }
                app.selected = 0;
            }
            _ => {}
        }
    }

    fn jump_search(&mut self, app: &mut App, dir: i64) {
        if app.last_search.is_empty() {
            return;
        }
        app.move_selection(dir);
    }

    fn handle_detail(&mut self, app: &mut App, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Esc | KeyCode::Char('h') | KeyCode::Backspace => {
                app.screen = Screen::List;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                app.detail_scroll = app.detail_scroll.saturating_add(1)
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.detail_scroll = app.detail_scroll.saturating_sub(1)
            }
            KeyCode::Char('d') if ctrl => app.detail_scroll = app.detail_scroll.saturating_add(10),
            KeyCode::Char('u') if ctrl => app.detail_scroll = app.detail_scroll.saturating_sub(10),
            KeyCode::Char('g') => app.detail_scroll = 0,
            KeyCode::Char('e') => self.open_editor(app),
            KeyCode::Char('f') => self.open_frontmatter_form(app),
            KeyCode::Char('s') => self.open_share(app),
            KeyCode::Char('x') | KeyCode::Char('D') => self.open_delete(app),
            KeyCode::Char('?') => {
                app.prev_screen = Screen::Detail;
                app.screen = Screen::Help;
            }
            _ => {}
        }
    }

    fn handle_help(&mut self, app: &mut App, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                app.screen = app.prev_screen;
            }
            _ => {}
        }
    }

    fn open_marketplace(&mut self, app: &mut App) {
        if self.market.is_none() {
            self.market = Some(Market::new());
        }
        app.prev_screen = Screen::List;
        app.screen = Screen::Marketplace;
    }

    fn handle_marketplace(&mut self, app: &mut App, key: KeyEvent) {
        let Some(market) = self.market.as_mut() else {
            app.screen = Screen::List;
            return;
        };
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match market.focus {
            MarketFocus::Search => match key.code {
                KeyCode::Esc => {
                    if market.query.is_empty() && market.results.is_empty() {
                        app.screen = Screen::List;
                    } else if !market.results.is_empty() {
                        market.focus = MarketFocus::Results;
                    } else {
                        market.query.clear();
                    }
                }
                KeyCode::Enter => market.start_search(),
                KeyCode::Backspace => {
                    market.query.pop();
                }
                KeyCode::Char(c) => market.query.push(c),
                KeyCode::Down | KeyCode::Tab if !market.results.is_empty() => {
                    market.focus = MarketFocus::Results;
                }
                _ => {}
            },
            MarketFocus::Results => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => app.screen = Screen::List,
                KeyCode::Char('/') => market.focus = MarketFocus::Search,
                KeyCode::Char('j') | KeyCode::Down => market.move_selection(1),
                KeyCode::Char('k') | KeyCode::Up => market.move_selection(-1),
                KeyCode::Char('d') if ctrl => market.move_selection(10),
                KeyCode::Char('u') if ctrl => market.move_selection(-10),
                KeyCode::Char('g') => market.selected = 0,
                KeyCode::Char('G') => market.selected = market.results.len().saturating_sub(1),
                KeyCode::Enter | KeyCode::Char('l') => market.start_fetch(),
                KeyCode::Char('i') => self.begin_install(app),
                _ => {}
            },
            MarketFocus::Detail => match key.code {
                KeyCode::Esc | KeyCode::Char('h') | KeyCode::Backspace => {
                    market.focus = MarketFocus::Results;
                }
                KeyCode::Char('q') => app.screen = Screen::List,
                KeyCode::Char('j') | KeyCode::Down => {
                    market.detail_scroll = market.detail_scroll.saturating_add(1)
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    market.detail_scroll = market.detail_scroll.saturating_sub(1)
                }
                KeyCode::Char('d') if ctrl => {
                    market.detail_scroll = market.detail_scroll.saturating_add(10)
                }
                KeyCode::Char('u') if ctrl => {
                    market.detail_scroll = market.detail_scroll.saturating_sub(10)
                }
                KeyCode::Char('g') => market.detail_scroll = 0,
                KeyCode::Char('i') => self.begin_install(app),
                _ => {}
            },
        }
    }

    fn begin_install(&mut self, app: &mut App) {
        let Some(market) = self.market.as_mut() else {
            return;
        };
        let Some(skill) = market.selected_skill() else {
            return;
        };
        let name = skill.name.clone();
        if market.detail_for(&name).is_none() {
            market.start_fetch();
            app.set_status("downloading skill — press i again once loaded", false);
            return;
        }
        let mut options = Vec::new();
        for provider in Provider::ALL {
            for scope in Scope::ALL {
                if app.registry.skills_dir(provider, scope).is_some() {
                    options.push((provider, scope));
                }
            }
        }
        if options.is_empty() {
            app.open_message("cannot install", "no writable skills directory found", true);
            return;
        }
        app.modal = Modal::InstallTarget {
            skill_name: name,
            options,
            cursor: 0,
        };
    }

    fn open_editor(&mut self, app: &mut App) {
        let Some(skill) = app.selected_skill() else {
            return;
        };
        let Some(instance) = skill.primary() else {
            app.open_message("no instance", "this skill has no editable file", true);
            return;
        };
        let skill_md = instance.skill_md.clone();
        let name = skill.name.clone();
        match crate::model::frontmatter::SkillDoc::from_file(&skill_md) {
            Ok(doc) => {
                self.editor = Some(Editor::new(skill_md, name, &doc.body));
                app.prev_screen = app.screen;
                app.screen = Screen::Editor;
            }
            Err(e) => app.open_message("cannot open", e.to_string(), true),
        }
    }

    fn handle_editor(&mut self, app: &mut App, key: KeyEvent) {
        let Some(editor) = self.editor.as_mut() else {
            app.screen = Screen::List;
            return;
        };
        match editor.handle_key(key) {
            EditorSignal::None => {}
            EditorSignal::Save => {
                self.save_editor(app);
            }
            EditorSignal::Quit => {
                self.editor = None;
                app.screen = app.prev_screen;
            }
            EditorSignal::SaveAndQuit => {
                if self.save_editor(app) {
                    self.editor = None;
                    app.screen = app.prev_screen;
                }
            }
        }
    }

    fn save_editor(&mut self, app: &mut App) -> bool {
        let Some(editor) = self.editor.as_mut() else {
            return false;
        };
        let body = editor.body();
        match actions::save_body(&editor.skill_md, &body) {
            Ok(()) => {
                editor.dirty = false;
                app.reload();
                app.set_status(format!("saved {}", editor.skill_name), false);
                true
            }
            Err(e) => {
                app.open_message("save failed", e.to_string(), true);
                false
            }
        }
    }

    fn open_create_form(&mut self, app: &mut App) {
        app.form = Some(FormState {
            kind: FormKind::Create,
            name: String::new(),
            description: String::new(),
            provider: Provider::Claude,
            scope: Scope::Global,
            field: FormField::Name,
            editing_skill: None,
            target_provider: Provider::Claude,
            target_scope: Scope::Global,
        });
        app.prev_screen = app.screen;
        app.screen = Screen::Form;
    }

    fn open_frontmatter_form(&mut self, app: &mut App) {
        let Some(skill) = app.selected_skill() else {
            return;
        };
        let Some(instance) = skill.primary() else {
            return;
        };
        app.form = Some(FormState {
            kind: FormKind::EditFrontmatter,
            name: skill.name.clone(),
            description: skill.description.clone(),
            provider: instance.provider,
            scope: instance.scope,
            field: FormField::Name,
            editing_skill: Some(skill.name.clone()),
            target_provider: instance.provider,
            target_scope: instance.scope,
        });
        app.prev_screen = app.screen;
        app.screen = Screen::Form;
    }

    fn handle_form(&mut self, app: &mut App, key: KeyEvent) {
        let Some(form) = app.form.as_mut() else {
            app.screen = Screen::List;
            return;
        };

        match key.code {
            KeyCode::Esc => {
                app.form = None;
                app.screen = app.prev_screen;
                return;
            }
            KeyCode::Tab | KeyCode::Down => {
                form.field = next_field(form.field, form.kind, 1);
                return;
            }
            KeyCode::BackTab | KeyCode::Up => {
                form.field = next_field(form.field, form.kind, -1);
                return;
            }
            KeyCode::Enter => {
                self.submit_form(app);
                return;
            }
            _ => {}
        }

        match form.field {
            FormField::Name => match key.code {
                KeyCode::Char(c) => form.name.push(c),
                KeyCode::Backspace => {
                    form.name.pop();
                }
                _ => {}
            },
            FormField::Description => match key.code {
                KeyCode::Char(c) => form.description.push(c),
                KeyCode::Backspace => {
                    form.description.pop();
                }
                _ => {}
            },
            FormField::Provider => {
                if matches!(
                    key.code,
                    KeyCode::Char(' ')
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('h')
                        | KeyCode::Char('l')
                ) {
                    form.provider = match form.provider {
                        Provider::Claude => Provider::Agents,
                        Provider::Agents => Provider::Claude,
                    };
                }
            }
            FormField::Scope => {
                if matches!(
                    key.code,
                    KeyCode::Char(' ')
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('h')
                        | KeyCode::Char('l')
                ) {
                    form.scope = match form.scope {
                        Scope::Global => Scope::Project,
                        Scope::Project => Scope::Global,
                    };
                }
            }
        }
    }

    fn submit_form(&mut self, app: &mut App) {
        let Some(form) = app.form.as_ref() else {
            return;
        };
        match form.kind {
            FormKind::Create => {
                let result = actions::create_skill(
                    &app.registry,
                    &form.name,
                    &form.description,
                    form.provider,
                    form.scope,
                );
                match result {
                    Ok(_) => {
                        let name = form.name.trim().to_string();
                        app.form = None;
                        app.screen = app.prev_screen;
                        app.reload();
                        select_by_name(app, &name);
                        app.set_status(format!("created {name}"), false);
                    }
                    Err(e) => app.open_message("create failed", e.to_string(), true),
                }
            }
            FormKind::EditFrontmatter => {
                let (Some(_), Some(skill)) = (form.editing_skill.as_ref(), app.selected_skill())
                else {
                    return;
                };
                let targets: Vec<_> = skill.instances.iter().map(|i| i.skill_md.clone()).collect();
                let name = form.name.clone();
                let description = form.description.clone();
                let mut err = None;
                for md in &targets {
                    if let Err(e) = actions::save_frontmatter(md, &name, &description) {
                        err = Some(e.to_string());
                        break;
                    }
                }
                match err {
                    None => {
                        let new_name = name.trim().to_string();
                        app.form = None;
                        app.screen = app.prev_screen;
                        app.reload();
                        select_by_name(app, &new_name);
                        app.set_status("updated frontmatter", false);
                    }
                    Some(e) => app.open_message("update failed", e, true),
                }
            }
        }
    }

    fn open_delete(&mut self, app: &mut App) {
        let Some(skill) = app.selected_skill() else {
            return;
        };
        let targets: Vec<(Provider, Scope)> = skill
            .instances
            .iter()
            .map(|i| (i.provider, i.scope))
            .collect();
        app.modal = Modal::ConfirmDelete {
            skill_name: skill.name.clone(),
            targets,
            cursor: 0,
        };
    }

    fn open_share(&mut self, app: &mut App) {
        let Some(skill) = app.selected_skill() else {
            return;
        };
        let mut options = Vec::new();
        for provider in Provider::ALL {
            for scope in Scope::ALL {
                if !skill.has(provider, scope) && app.registry.skills_dir(provider, scope).is_some()
                {
                    options.push((provider, scope));
                }
            }
        }
        if options.is_empty() {
            app.open_message(
                "already everywhere",
                "this skill already exists in every provider and scope",
                false,
            );
            return;
        }
        app.modal = Modal::Share {
            skill_name: skill.name.clone(),
            options,
            cursor: 0,
            method_choice: None,
        };
    }

    fn handle_modal(&mut self, app: &mut App, key: KeyEvent) {
        let mut modal = std::mem::replace(&mut app.modal, Modal::None);
        match &mut modal {
            Modal::Message { .. } => match key.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => {}
                _ => app.modal = modal,
            },
            Modal::ConfirmDelete {
                skill_name,
                targets,
                cursor,
            } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('n') => {}
                KeyCode::Char('j') | KeyCode::Down => {
                    *cursor = (*cursor + 1) % (targets.len() + 1);
                    app.modal = modal;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *cursor = (*cursor + targets.len()) % (targets.len() + 1);
                    app.modal = modal;
                }
                KeyCode::Char(' ') | KeyCode::Enter | KeyCode::Char('y') => {
                    let confirm_all = key.code == KeyCode::Char('y') || *cursor == targets.len();
                    let chosen: Vec<(Provider, Scope)> = if confirm_all {
                        targets.clone()
                    } else {
                        vec![targets[*cursor]]
                    };
                    self.perform_delete(app, skill_name, &chosen);
                }
                _ => app.modal = modal,
            },
            Modal::Share {
                skill_name,
                options,
                cursor,
                method_choice,
            } => {
                if method_choice.is_none() {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {}
                        KeyCode::Char('j') | KeyCode::Down => {
                            *cursor = (*cursor + 1) % options.len();
                            app.modal = modal;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            *cursor = (*cursor + options.len() - 1) % options.len();
                            app.modal = modal;
                        }
                        KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('l') => {
                            *method_choice = Some(0);
                            app.modal = modal;
                        }
                        _ => app.modal = modal,
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => {
                            *method_choice = None;
                            app.modal = modal;
                        }
                        KeyCode::Char('h')
                        | KeyCode::Char('l')
                        | KeyCode::Left
                        | KeyCode::Right
                        | KeyCode::Char('j')
                        | KeyCode::Char('k')
                        | KeyCode::Tab => {
                            let cur = method_choice.unwrap_or(0);
                            *method_choice = Some(if cur == 0 { 1 } else { 0 });
                            app.modal = modal;
                        }
                        KeyCode::Char('c') => {
                            *method_choice = Some(0);
                            self.perform_share(
                                app,
                                skill_name,
                                options[*cursor],
                                ShareMethod::Copy,
                            );
                        }
                        KeyCode::Char('s') => {
                            self.perform_share(
                                app,
                                skill_name,
                                options[*cursor],
                                ShareMethod::Symlink,
                            );
                        }
                        KeyCode::Enter => {
                            let method = if method_choice == &Some(1) {
                                ShareMethod::Symlink
                            } else {
                                ShareMethod::Copy
                            };
                            self.perform_share(app, skill_name, options[*cursor], method);
                        }
                        _ => app.modal = modal,
                    }
                }
            }
            Modal::InstallTarget {
                skill_name,
                options,
                cursor,
            } => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {}
                KeyCode::Char('j') | KeyCode::Down => {
                    *cursor = (*cursor + 1) % options.len();
                    app.modal = modal;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *cursor = (*cursor + options.len() - 1) % options.len();
                    app.modal = modal;
                }
                KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('l') => {
                    let (provider, scope) = options[*cursor];
                    let name = skill_name.clone();
                    self.perform_install(app, &name, provider, scope, false);
                }
                _ => app.modal = modal,
            },
            Modal::ConfirmInstallOverwrite {
                skill_name,
                provider,
                scope,
            } => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    let (name, provider, scope) = (skill_name.clone(), *provider, *scope);
                    self.perform_install(app, &name, provider, scope, true);
                }
                KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('q') => {}
                _ => app.modal = modal,
            },
            Modal::None => {}
        }
    }

    fn perform_install(
        &mut self,
        app: &mut App,
        skill_name: &str,
        provider: Provider,
        scope: Scope,
        overwrite: bool,
    ) {
        let Some(market) = self.market.as_ref() else {
            return;
        };
        let Some(content) = market.detail_for(skill_name) else {
            app.open_message(
                "not downloaded",
                "open the skill first so its files can be fetched",
                true,
            );
            return;
        };
        match actions::install_skill(
            &app.registry,
            skill_name,
            content,
            provider,
            scope,
            overwrite,
        ) {
            Ok(_) => {
                app.reload();
                app.set_status(
                    format!("installed {skill_name} → {provider}/{scope}"),
                    false,
                );
            }
            Err(e) => {
                let msg = e.to_string();
                if !overwrite && msg.contains("already exists") {
                    app.modal = Modal::ConfirmInstallOverwrite {
                        skill_name: skill_name.to_string(),
                        provider,
                        scope,
                    };
                } else {
                    app.open_message("install failed", msg, true);
                }
            }
        }
    }

    fn perform_delete(&mut self, app: &mut App, skill_name: &str, targets: &[(Provider, Scope)]) {
        let Some(skill) = app.registry.skills.iter().find(|s| s.name == skill_name) else {
            return;
        };
        let dirs: Vec<_> = targets
            .iter()
            .filter_map(|(p, s)| skill.instance(*p, *s).map(|i| i.dir.clone()))
            .collect();
        match actions::delete_instances(&dirs) {
            Ok(()) => {
                app.reload();
                app.set_status(format!("deleted {} instance(s)", dirs.len()), false);
            }
            Err(e) => app.open_message("delete failed", e.to_string(), true),
        }
    }

    fn perform_share(
        &mut self,
        app: &mut App,
        skill_name: &str,
        target: (Provider, Scope),
        method: ShareMethod,
    ) {
        let source_dir = app
            .registry
            .skills
            .iter()
            .find(|s| s.name == skill_name)
            .and_then(|s| s.primary())
            .map(|i| i.dir.clone());
        let Some(source_dir) = source_dir else {
            return;
        };
        let result = actions::share_skill(
            &app.registry,
            &source_dir,
            target.0,
            target.1,
            skill_name,
            method,
        );
        match result {
            Ok(_) => {
                app.reload();
                select_by_name(app, skill_name);
                app.set_status(format!("shared to {}/{}", target.0, target.1), false);
            }
            Err(e) => app.open_message("share failed", e.to_string(), true),
        }
    }
}

fn next_field(field: FormField, kind: FormKind, dir: i64) -> FormField {
    let order: &[FormField] = match kind {
        FormKind::Create => &[
            FormField::Name,
            FormField::Description,
            FormField::Provider,
            FormField::Scope,
        ],
        FormKind::EditFrontmatter => &[FormField::Name, FormField::Description],
    };
    let idx = order.iter().position(|f| *f == field).unwrap_or(0) as i64;
    let len = order.len() as i64;
    let next = (idx + dir + len) % len;
    order[next as usize]
}

fn select_by_name(app: &mut App, name: &str) {
    if let Some(pos) = app
        .filtered_indices()
        .iter()
        .position(|&i| app.registry.skills[i].name == name)
    {
        app.selected = pos;
    }
}
