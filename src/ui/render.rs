use super::app::{App, FormField, FormKind, Modal, Screen, SkillGroup};
use super::editor::{Editor, VimMode};
use super::events::Controller;
use super::market::{Market, MarketFocus};
use crate::market::MarketSkill;
use crate::model::{HarnessFile, Provider, Scope, Skill};
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph, Wrap,
};
use ratatui::Frame;

const ACCENT: Color = Color::Cyan;
const ACCENT2: Color = Color::Green;
const WARN: Color = Color::Yellow;
const ERR: Color = Color::Red;
const CLAUDE_COLOR: Color = Color::Magenta;
const AGENTS_COLOR: Color = Color::Blue;
const HL_BG: Color = Color::Indexed(8);
const FG: Color = Color::Reset;
const DIM: Color = Color::Gray;
const FAINT: Color = Color::DarkGray;
const BADGE_FG: Color = Color::Black;

pub fn render(f: &mut Frame, app: &App, controller: &Controller) {
    let editor = controller.editor.as_ref();
    let market = controller.market.as_ref();
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(area);

    render_header(f, app, chunks[0]);

    match app.screen {
        Screen::List => render_list_screen(f, app, chunks[1]),
        Screen::Detail => render_detail(f, app, chunks[1]),
        Screen::Editor => {
            if let Some(ed) = editor {
                render_editor(f, ed, chunks[1]);
            }
        }
        Screen::Form => {
            render_list_screen(f, app, chunks[1]);
            render_form_modal(f, app, area);
        }
        Screen::Help => render_help(f, area),
        Screen::Marketplace => {
            if let Some(m) = market {
                render_marketplace(f, m, chunks[1]);
            }
        }
        Screen::Harness | Screen::Commands => render_harness(f, app, chunks[1]),
    }

    render_status(f, app, editor, market, chunks[2]);

    match &app.modal {
        Modal::None => {}
        Modal::ConfirmDelete { .. } => render_delete_modal(f, app, area),
        Modal::Share { .. } => render_share_modal(f, app, area),
        Modal::Message { .. } => render_message_modal(f, app, area),
        Modal::InstallTarget { .. } => render_install_modal(f, app, area),
        Modal::ConfirmInstallOverwrite { .. } => render_install_overwrite_modal(f, app, area),
        Modal::LinkHarness { .. } => render_link_harness_modal(f, app, area),
        Modal::ConfirmDeleteHarness { .. } => render_delete_harness_modal(f, app, area),
        Modal::CreateCommand { .. } => render_create_command_modal(f, app, area),
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    if app.screen == Screen::Marketplace {
        let title = Line::from(vec![
            Span::styled(
                " skills.sh ",
                Style::default()
                    .bg(ACCENT2)
                    .fg(BADGE_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("marketplace", Style::default().fg(DIM)),
        ]);
        f.render_widget(Paragraph::new(title), area);
        return;
    }
    if app.screen == Screen::Harness || app.screen == Screen::Commands {
        let count = app.harness_view_len();
        let (badge, blurb) = if app.screen == Screen::Commands {
            (
                " commands ",
                format!("{count} command files · global & project"),
            )
        } else {
            (
                " harness ",
                format!("{count} memory files · CLAUDE.md / AGENTS.md, global & project"),
            )
        };
        let title = Line::from(vec![
            Span::styled(
                badge,
                Style::default()
                    .bg(WARN)
                    .fg(BADGE_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(blurb, Style::default().fg(DIM)),
        ]);
        f.render_widget(Paragraph::new(title), area);
        return;
    }
    let count = app.visible_count();
    let total = app.registry.skills.len();
    let title = Line::from(vec![
        Span::styled(
            " skillsdash ",
            Style::default()
                .bg(ACCENT)
                .fg(BADGE_FG)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(format!("{count}/{total} skills"), Style::default().fg(DIM)),
        Span::raw("  "),
        Span::styled(
            format!("group: {}", if app.grouped { "on" } else { "off" }),
            Style::default().fg(if app.grouped { ACCENT2 } else { DIM }),
        ),
        Span::raw("  "),
        Span::styled("m marketplace", Style::default().fg(ACCENT2)),
        Span::raw("  "),
        Span::styled("h harness", Style::default().fg(WARN)),
        Span::raw("  "),
        Span::styled("c commands", Style::default().fg(WARN)),
    ]);
    f.render_widget(Paragraph::new(title), area);
}

fn render_list_screen(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_list(f, app, cols[0]);
    render_side_preview(f, app, cols[1]);
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let sections = app.grouped_sections();

    if !app.grouped {
        // Single flat box, no group boxes.
        let rows = &sections[0].1;
        render_section_box(
            f,
            app,
            area,
            " skills ".to_string(),
            ACCENT,
            rows,
            true,
            true,
        );
        return;
    }

    // One bordered box per group, always both, stacked vertically. Height is
    // shared proportionally to each group's skill count so a small (or empty)
    // group stays small.
    let constraints: Vec<Constraint> = sections
        .iter()
        .map(|(_, rows)| Constraint::Fill((rows.len().max(1)) as u16))
        .collect();
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (idx, ((group, rows), slot)) in sections.iter().zip(slots.iter()).enumerate() {
        let color = group_color(*group);
        let title = format!(" {} ({}) ", group.heading(), rows.len());
        let focused = app.focused_group == *group;
        // Show the live search query on the top box only, to avoid repeating it.
        render_section_box(f, app, *slot, title, color, rows, idx == 0, focused);
    }
}

/// Draw one bordered list box for a group. The border brightens when the box
/// is focused; the selected row is highlighted only when this box is focused
/// and holds the selection. Empty boxes show a hint. When searching,
/// `show_search` swaps the group title for the live query.
#[allow(clippy::too_many_arguments)]
fn render_section_box(
    f: &mut Frame,
    app: &App,
    area: Rect,
    title: String,
    color: Color,
    rows: &[(usize, usize)],
    show_search: bool,
    focused: bool,
) {
    let items: Vec<ListItem> = rows
        .iter()
        .map(|&(_, registry_index)| list_item(&app.registry.skills[registry_index]))
        .collect();

    let mut state = ListState::default();
    if focused {
        if let Some(pos) = rows
            .iter()
            .position(|&(skill_index, _)| skill_index == app.selected)
        {
            state.select(Some(pos));
        }
    }

    let title = if show_search && app.search_active {
        search_title(app)
    } else {
        title
    };

    let block = list_block(app, &title, color, focused);
    f.render_widget(block.clone(), area);

    if rows.is_empty() {
        let msg = if app.search.as_deref().unwrap_or("").is_empty() {
            "no skills here — press a to create one"
        } else {
            "no matches"
        };
        let inner = block.inner(area);
        if inner.height > 0 {
            f.render_widget(Paragraph::new(msg).style(Style::default().fg(DIM)), inner);
        }
        return;
    }

    // No highlight symbol: it would reserve a left gutter and shift every row
    // in the focused box right. The background highlight marks the active item.
    let list = List::default()
        .items(items)
        .block(block)
        .highlight_style(Style::default().bg(HL_BG).add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, area, &mut state);
}

fn list_block(app: &App, title: &str, color: Color, focused: bool) -> Block<'static> {
    let color = if app.search_active {
        WARN
    } else if focused {
        color
    } else {
        FAINT
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            title.to_string(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ))
}

fn search_title(app: &App) -> String {
    match (&app.search, app.search_active) {
        (Some(q), true) => format!(" search: {q}▏"),
        (Some(q), false) if !q.is_empty() => format!(" /{q} "),
        _ => " skills ".to_string(),
    }
}

fn group_color(group: SkillGroup) -> Color {
    match group {
        SkillGroup::Project => ACCENT2,
        SkillGroup::Global => ACCENT,
    }
}

fn list_item(skill: &Skill) -> ListItem<'static> {
    let mut spans = vec![Span::styled(
        skill.name.clone(),
        Style::default().add_modifier(Modifier::BOLD),
    )];
    spans.push(Span::raw("  "));
    spans.extend(provider_badges(skill));
    ListItem::new(Line::from(spans))
}

fn provider_badges(skill: &Skill) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    for provider in Provider::ALL {
        let in_global = skill.has(provider, Scope::Global);
        let in_project = skill.has(provider, Scope::Project);
        if !in_global && !in_project {
            continue;
        }
        let color = provider_color(provider);
        let mut label = provider.label().to_string();
        if in_project {
            label.push('*');
        }
        spans.push(Span::styled(
            format!(" {label} "),
            Style::default().fg(BADGE_FG).bg(color),
        ));
        spans.push(Span::raw(" "));
    }
    spans
}

fn provider_color(provider: Provider) -> Color {
    match provider {
        Provider::Claude => CLAUDE_COLOR,
        Provider::Agents => AGENTS_COLOR,
    }
}

fn render_side_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(FAINT))
        .padding(Padding::horizontal(1))
        .title(Span::styled(" preview ", Style::default().fg(DIM)));

    let Some(skill) = app.selected_skill() else {
        f.render_widget(block, area);
        return;
    };

    let mut lines = vec![
        Line::from(Span::styled(
            skill.name.clone(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
    ];

    for line in wrap_desc(&skill.description) {
        lines.push(Line::from(Span::styled(line, Style::default().fg(FG))));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "available in:",
        Style::default().fg(DIM),
    )));
    for instance in &skill.instances {
        let mut spans = vec![
            Span::raw("  "),
            Span::styled(
                format!("{}", instance.provider),
                Style::default()
                    .fg(provider_color(instance.provider))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" / {}", instance.scope), Style::default().fg(DIM)),
        ];
        if instance.is_symlink {
            spans.push(Span::styled("  ↪ symlink", Style::default().fg(WARN)));
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "enter/l  open   e edit   f frontmatter",
        Style::default().fg(DIM),
    )));
    lines.push(Line::from(Span::styled(
        "s share  x delete",
        Style::default().fg(DIM),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_detail(f: &mut Frame, app: &App, area: Rect) {
    let Some(skill) = app.selected_skill() else {
        app_empty(f, area, "no skill selected");
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    let mut meta = vec![
        Line::from(Span::styled(
            skill.name.clone(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled("description", Style::default().fg(DIM))),
    ];
    for line in wrap_desc(&skill.description) {
        meta.push(Line::from(line));
    }
    meta.push(Line::from(""));
    meta.push(Line::from(Span::styled(
        "instances",
        Style::default().fg(DIM),
    )));
    for instance in &skill.instances {
        meta.push(Line::from(vec![
            Span::styled(
                format!("{}", instance.provider),
                Style::default()
                    .fg(provider_color(instance.provider))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" / {}", instance.scope), Style::default().fg(DIM)),
            if instance.is_symlink {
                Span::styled("  ↪", Style::default().fg(WARN))
            } else {
                Span::raw("")
            },
        ]));
        meta.push(Line::from(Span::styled(
            format!("  {}", short_path(&instance.dir.display().to_string())),
            Style::default().fg(DIM),
        )));
    }

    let meta_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .padding(Padding::horizontal(1))
        .title(" details ");
    f.render_widget(
        Paragraph::new(meta)
            .block(meta_block)
            .wrap(Wrap { trim: false }),
        cols[0],
    );

    let body = skill
        .primary()
        .and_then(|i| crate::model::frontmatter::SkillDoc::from_file(&i.skill_md).ok())
        .map(|d| d.body)
        .unwrap_or_default();

    let body_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .padding(Padding::horizontal(1))
        .title(" SKILL.md ");

    f.render_widget(
        Paragraph::new(markdown_text(&body))
            .block(body_block)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0)),
        cols[1],
    );
}

fn render_editor(f: &mut Frame, editor: &Editor, area: Rect) {
    let mode_color = match editor.mode {
        VimMode::Normal => ACCENT,
        VimMode::Insert => ACCENT2,
        VimMode::Command => WARN,
    };
    let dirty = if editor.dirty { " ●" } else { "" };
    let title = format!(" {} — {}{} ", editor.skill_name, editor.file_label, dirty);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(mode_color))
        .title(Span::styled(title, Style::default().fg(mode_color)));

    let mut textarea = editor.textarea.clone();
    textarea.set_block(block);
    textarea.set_cursor_style(Style::default().bg(mode_color).fg(BADGE_FG));
    textarea.set_line_number_style(Style::default().fg(DIM));
    f.render_widget(&textarea, area);
}

fn render_form_modal(f: &mut Frame, app: &App, area: Rect) {
    let Some(form) = &app.form else { return };
    let width = 64.min(area.width.saturating_sub(4));
    let height = match form.kind {
        FormKind::Create => 13,
        FormKind::EditFrontmatter => 9,
    };
    let rect = centered(area, width, height);
    f.render_widget(Clear, rect);

    let title = match form.kind {
        FormKind::Create => " new skill ",
        FormKind::EditFrontmatter => " edit frontmatter ",
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            title,
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));

    let mut lines = Vec::new();
    lines.push(field_line(
        "name",
        &form.name,
        form.field == FormField::Name,
    ));
    lines.push(Line::from(""));
    lines.push(field_line(
        "description",
        &form.description,
        form.field == FormField::Description,
    ));

    if form.kind == FormKind::Create {
        lines.push(Line::from(""));
        lines.push(checkbox_row(
            "provider",
            &Provider::ALL.map(|p| p.label()),
            &form.providers,
            form.provider_cursor,
            form.field == FormField::Provider,
        ));
        lines.push(checkbox_row(
            "scope",
            &Scope::ALL.map(|s| s.label()),
            &form.scopes,
            form.scope_cursor,
            form.field == FormField::Scope,
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "tab next · ←/→ move · space toggle · enter save · esc cancel",
        Style::default().fg(DIM),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn field_line(label: &str, value: &str, active: bool) -> Line<'static> {
    let marker = if active { "▸ " } else { "  " };
    let cursor = if active { "▏" } else { "" };
    let value_style = if active {
        Style::default().fg(FG)
    } else {
        Style::default().fg(DIM)
    };
    Line::from(vec![
        Span::styled(marker, Style::default().fg(ACCENT)),
        Span::styled(format!("{label}: "), Style::default().fg(DIM)),
        Span::styled(format!("{value}{cursor}"), value_style),
    ])
}

/// A multi-select row: a label followed by one checkbox chip per option.
/// `checked` marks selected options; `cursor` is the focused chip; `active`
/// means this row currently has keyboard focus.
fn checkbox_row(
    label: &str,
    options: &[&str],
    checked: &[bool],
    cursor: usize,
    active: bool,
) -> Line<'static> {
    let marker = if active { "▸ " } else { "  " };
    let mut spans = vec![
        Span::styled(marker, Style::default().fg(ACCENT)),
        Span::styled(format!("{label}: "), Style::default().fg(DIM)),
    ];
    for (i, opt) in options.iter().enumerate() {
        let is_checked = checked.get(i).copied().unwrap_or(false);
        let on_cursor = active && i == cursor;
        let mark = if is_checked { "✓" } else { " " };
        let text = format!("[{mark}] {opt}");
        let style = if on_cursor {
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD)
        } else if is_checked {
            Style::default().fg(ACCENT2)
        } else {
            Style::default().fg(DIM)
        };
        spans.push(Span::styled(text, style));
        if i + 1 < options.len() {
            spans.push(Span::raw("  "));
        }
    }
    Line::from(spans)
}

fn render_delete_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::ConfirmDelete {
        skill_name,
        targets,
        cursor,
    } = &app.modal
    else {
        return;
    };
    let height = (targets.len() as u16) + 8;
    let rect = centered(
        area,
        62.min(area.width.saturating_sub(4)),
        height.min(area.height.saturating_sub(2)),
    );
    f.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ERR))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " delete skill ",
            Style::default().fg(ERR).add_modifier(Modifier::BOLD),
        ));

    let mut lines = vec![
        Line::from(vec![
            Span::raw("remove "),
            Span::styled(
                skill_name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" from:"),
        ]),
        Line::from(""),
    ];
    for (idx, (provider, scope)) in targets.iter().enumerate() {
        let selected = *cursor == idx;
        lines.push(Line::from(vec![
            Span::styled(if selected { "▸ " } else { "  " }, Style::default().fg(ERR)),
            Span::styled(
                format!("{provider} / {scope}"),
                if selected {
                    Style::default().fg(FG).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DIM)
                },
            ),
        ]));
    }
    let all_selected = *cursor == targets.len();
    lines.push(Line::from(vec![
        Span::styled(
            if all_selected { "▸ " } else { "  " },
            Style::default().fg(ERR),
        ),
        Span::styled(
            "all instances",
            if all_selected {
                Style::default().fg(ERR).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DIM)
            },
        ),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "j/k move · enter delete selection · y delete all · esc cancel",
        Style::default().fg(DIM),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn render_share_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::Share {
        skill_name,
        options,
        cursor,
        method_choice,
    } = &app.modal
    else {
        return;
    };
    let height = (options.len() as u16) + 11;
    let rect = centered(
        area,
        60.min(area.width.saturating_sub(4)),
        height.min(area.height.saturating_sub(2)),
    );
    f.render_widget(Clear, rect);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT2))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " share skill ",
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        ));

    let mut lines = vec![
        Line::from(vec![
            Span::raw("make "),
            Span::styled(
                skill_name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" available in:"),
        ]),
        Line::from(""),
    ];
    for (idx, (provider, scope)) in options.iter().enumerate() {
        let selected = *cursor == idx;
        lines.push(Line::from(vec![
            Span::styled(
                if selected { "▸ " } else { "  " },
                Style::default().fg(ACCENT2),
            ),
            Span::styled(
                format!("{provider} / {scope}"),
                if selected {
                    Style::default().fg(FG).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DIM)
                },
            ),
        ]));
    }
    lines.push(Line::from(""));

    if method_choice.is_none() {
        lines.push(Line::from(Span::styled(
            "j/k move · enter choose method · esc cancel",
            Style::default().fg(DIM),
        )));
    } else {
        let choice = method_choice.unwrap_or(0);
        lines.push(Line::from(Span::styled(
            "method:",
            Style::default().fg(DIM),
        )));
        lines.push(Line::from(vec![
            method_chip("copy", choice == 0),
            Span::raw("  "),
            method_chip("symlink", choice == 1),
        ]));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "←/→ switch · enter confirm · c copy · s symlink · esc back",
            Style::default().fg(DIM),
        )));
    }

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn method_chip(label: &str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            format!(" {label} "),
            Style::default()
                .bg(ACCENT2)
                .fg(BADGE_FG)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(DIM))
    }
}

fn render_message_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::Message {
        title,
        body,
        is_error,
    } = &app.modal
    else {
        return;
    };
    let color = if *is_error { ERR } else { ACCENT };
    let rect = centered(area, 60.min(area.width.saturating_sub(4)), 9);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    let lines = vec![
        Line::from(body.clone()),
        Line::from(""),
        Line::from(Span::styled("enter/esc dismiss", Style::default().fg(DIM))),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn render_marketplace(f: &mut Frame, market: &Market, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    render_market_search(f, market, rows[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(rows[1]);

    render_market_results(f, market, cols[0]);
    render_market_detail(f, market, cols[1]);
}

fn render_market_search(f: &mut Frame, market: &Market, area: Rect) {
    let focused = market.focus == MarketFocus::Search;
    let cursor = if focused { "▏" } else { "" };
    let text = if market.query.is_empty() && !focused {
        Span::styled("type to search skills.sh…", Style::default().fg(DIM))
    } else {
        Span::styled(format!("{}{cursor}", market.query), Style::default().fg(FG))
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { WARN } else { DIM }))
        .padding(Padding::horizontal(1))
        .title(Span::styled(
            " search ",
            Style::default().fg(if focused { WARN } else { DIM }),
        ));
    f.render_widget(Paragraph::new(Line::from(text)).block(block), area);
}

fn render_market_results(f: &mut Frame, market: &Market, area: Rect) {
    let focused = market.focus == MarketFocus::Results;
    let title = if market.results.is_empty() {
        " results ".to_string()
    } else {
        format!(" results ({}) ", market.results.len())
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { ACCENT } else { DIM }))
        .title(Span::styled(
            title,
            Style::default().fg(if focused { ACCENT } else { DIM }),
        ));

    if market.results.is_empty() {
        let msg = if market.last_query.is_empty() {
            "search for a skill to get started"
        } else if market.searching {
            "searching…"
        } else {
            "no results"
        };
        f.render_widget(
            Paragraph::new(msg)
                .style(Style::default().fg(DIM))
                .block(block),
            area,
        );
        return;
    }

    let items: Vec<ListItem> = market.results.iter().map(market_result_item).collect();
    let mut state = ListState::default();
    state.select(Some(market.selected.min(market.results.len() - 1)));
    let list = List::default()
        .items(items)
        .block(block)
        .highlight_style(Style::default().bg(HL_BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("▌ ");
    f.render_stateful_widget(list, area, &mut state);
}

fn market_result_item(skill: &MarketSkill) -> ListItem<'static> {
    let line1 = Line::from(vec![
        Span::styled(
            skill.name.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("↓{}", human_count(skill.installs)),
            Style::default().fg(ACCENT2),
        ),
    ]);
    let line2 = Line::from(Span::styled(
        format!("  {}", skill.source),
        Style::default().fg(DIM),
    ));
    ListItem::new(vec![line1, line2])
}

fn render_market_detail(f: &mut Frame, market: &Market, area: Rect) {
    let focused = market.focus == MarketFocus::Detail;
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focused { ACCENT } else { DIM }))
        .padding(Padding::horizontal(1))
        .title(Span::styled(
            " skill ",
            Style::default().fg(if focused { ACCENT } else { DIM }),
        ));

    let Some(skill) = market.selected_skill() else {
        f.render_widget(
            Paragraph::new("select a result")
                .style(Style::default().fg(DIM))
                .block(block),
            area,
        );
        return;
    };

    if market.fetching {
        f.render_widget(
            Paragraph::new(format!(
                "{} loading {}…",
                market.spinner_frame(),
                skill.name
            ))
            .style(Style::default().fg(WARN))
            .block(block),
            area,
        );
        return;
    }

    let mut header = vec![
        Line::from(Span::styled(
            skill.name.clone(),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("source: ", Style::default().fg(DIM)),
            Span::styled(skill.source.clone(), Style::default().fg(FG)),
        ]),
        Line::from(vec![
            Span::styled("installs: ", Style::default().fg(DIM)),
            Span::styled(human_count(skill.installs), Style::default().fg(ACCENT2)),
        ]),
        Line::from(""),
    ];

    match market
        .detail
        .as_ref()
        .filter(|_| market.detail_for(&skill.name).is_some())
    {
        Some(content) => {
            let body = content
                .skill_md()
                .map(|f| String::from_utf8_lossy(&f.bytes).to_string())
                .unwrap_or_default();
            header.push(Line::from(Span::styled(
                "SKILL.md:",
                Style::default().fg(DIM),
            )));
            let mut lines = header;
            lines.extend(markdown_text(&body).lines);
            f.render_widget(
                Paragraph::new(lines)
                    .block(block)
                    .wrap(Wrap { trim: false })
                    .scroll((market.detail_scroll, 0)),
                area,
            );
        }
        None => {
            header.push(Line::from(Span::styled(
                "press enter to load the skill, i to install",
                Style::default().fg(DIM),
            )));
            f.render_widget(
                Paragraph::new(header)
                    .block(block)
                    .wrap(Wrap { trim: false }),
                area,
            );
        }
    }
}

fn render_harness(f: &mut Frame, app: &App, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_harness_list(f, app, cols[0]);
    render_harness_preview(f, app, cols[1]);
}

fn render_harness_list(f: &mut Frame, app: &App, area: Rect) {
    let kind = app.harness_kind();
    let sections = app.harness_scope_sections(kind);

    let constraints: Vec<Constraint> = sections
        .iter()
        .map(|(_, rows)| Constraint::Fill((rows.len().max(1)) as u16))
        .collect();
    let slots = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for ((scope, rows), slot) in sections.iter().zip(slots.iter()) {
        render_harness_section_box(f, app, *slot, *scope, rows);
    }
}

fn render_harness_section_box(
    f: &mut Frame,
    app: &App,
    area: Rect,
    scope: Scope,
    rows: &[(usize, &HarnessFile)],
) {
    let commands = app.screen == Screen::Commands;
    let kind_label = if commands { "commands" } else { "memory" };
    let color = match scope {
        Scope::Global => ACCENT,
        Scope::Project => ACCENT2,
    };
    let title = format!(" {} {} ({}) ", scope.label(), kind_label, rows.len());
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color))
        .title(Span::styled(
            title,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block.clone(), area);

    if rows.is_empty() {
        let msg = if commands {
            "no command files — press a to create one"
        } else {
            "no memory files"
        };
        let inner = block.inner(area);
        if inner.height > 0 {
            f.render_widget(Paragraph::new(msg).style(Style::default().fg(DIM)), inner);
        }
        return;
    }

    let items: Vec<ListItem> = rows.iter().map(|(_, file)| harness_item(file)).collect();
    let mut state = ListState::default();
    if let Some(local) = rows
        .iter()
        .position(|(pos, _)| *pos == app.harness_selected)
    {
        state.select(Some(local));
    }
    let list = List::default()
        .items(items)
        .block(block)
        .highlight_style(Style::default().bg(HL_BG).add_modifier(Modifier::BOLD))
        .highlight_symbol("▌ ");
    f.render_stateful_widget(list, area, &mut state);
}

fn harness_item(file: &HarnessFile) -> ListItem<'static> {
    let name_style = if file.exists {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(DIM).add_modifier(Modifier::BOLD)
    };
    let mut spans = vec![
        Span::styled(file.name.clone(), name_style),
        Span::raw("  "),
        Span::styled(
            format!(" {} ", file.provider.label()),
            Style::default()
                .fg(BADGE_FG)
                .bg(provider_color(file.provider)),
        ),
    ];
    if file.is_symlink {
        spans.push(Span::styled("  ↪ link", Style::default().fg(WARN)));
    } else if !file.exists {
        spans.push(Span::styled("  (none)", Style::default().fg(DIM)));
    }
    ListItem::new(Line::from(spans))
}

fn render_harness_preview(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(FAINT))
        .padding(Padding::horizontal(1))
        .title(Span::styled(" preview ", Style::default().fg(DIM)));

    let Some(file) = app.harness_selected_file() else {
        f.render_widget(block, area);
        return;
    };

    let mut lines = vec![
        Line::from(Span::styled(
            file.name.clone(),
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(
                format!("{}", file.provider),
                Style::default()
                    .fg(provider_color(file.provider))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!(" / {}", file.scope), Style::default().fg(DIM)),
        ]),
        Line::from(Span::styled(
            short_path(&file.path.display().to_string()),
            Style::default().fg(DIM),
        )),
        Line::from(""),
    ];

    if file.is_symlink {
        lines.push(Line::from(Span::styled(
            "↪ symlink",
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        )));
        if let Some(target) = &file.link_target {
            lines.push(Line::from(Span::styled(
                format!("  → {}", short_path(&target.display().to_string())),
                Style::default().fg(DIM),
            )));
        }
        lines.push(Line::from(""));
    } else if !file.exists {
        lines.push(Line::from(Span::styled(
            "does not exist yet",
            Style::default().fg(FAINT),
        )));
        lines.push(Line::from(Span::styled(
            "press e to create it",
            Style::default().fg(DIM),
        )));
        lines.push(Line::from(""));
    }

    if file.exists {
        if let Ok(body) = std::fs::read_to_string(&file.path) {
            lines.push(Line::from(Span::styled(
                "contents:",
                Style::default().fg(DIM),
            )));
            for raw in body.lines().take(12) {
                lines.push(Line::from(Span::styled(
                    raw.to_string(),
                    Style::default().fg(FG),
                )));
            }
            if body.lines().count() > 12 {
                lines.push(Line::from(Span::styled("  …", Style::default().fg(FAINT))));
            }
            lines.push(Line::from(""));
        }
    }

    let other = match file.provider {
        Provider::Claude => "agents",
        Provider::Agents => "claude",
    };
    lines.push(Line::from(Span::styled(
        format!("e edit   s link → {other}   x delete"),
        Style::default().fg(DIM),
    )));

    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_link_harness_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::LinkHarness {
        source_label,
        target_label,
        ..
    } = &app.modal
    else {
        return;
    };
    let rect = centered(area, 64.min(area.width.saturating_sub(4)), 11);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT2))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " link harness file ",
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        ));
    let lines = vec![
        Line::from(Span::styled(
            "make a symlink so both providers share one file:",
            Style::default().fg(DIM),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("source: ", Style::default().fg(DIM)),
            Span::styled(source_label.clone(), Style::default().fg(FG)),
        ]),
        Line::from(vec![
            Span::styled("link:   ", Style::default().fg(DIM)),
            Span::styled(target_label.clone(), Style::default().fg(ACCENT2)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "y / enter link · n / esc cancel",
            Style::default().fg(DIM),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn render_delete_harness_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::ConfirmDeleteHarness {
        label, is_symlink, ..
    } = &app.modal
    else {
        return;
    };
    let rect = centered(area, 60.min(area.width.saturating_sub(4)), 9);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ERR))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " delete harness file ",
            Style::default().fg(ERR).add_modifier(Modifier::BOLD),
        ));
    let what = if *is_symlink {
        "remove the symlink"
    } else {
        "delete the file"
    };
    let lines = vec![
        Line::from(vec![
            Span::raw(format!("{what} ")),
            Span::styled(label.clone(), Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "y / enter delete · n / esc cancel",
            Style::default().fg(DIM),
        )),
    ];
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn render_create_command_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::CreateCommand {
        name,
        provider,
        scope,
    } = &app.modal
    else {
        return;
    };
    let rect = centered(area, 60.min(area.width.saturating_sub(4)), 11);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(WARN))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " new command ",
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        ));
    let mut lines = vec![
        field_line("name", name, true),
        Line::from(vec![
            Span::styled("      /", Style::default().fg(DIM)),
            Span::styled(format!("{}.md", name.trim()), Style::default().fg(FAINT)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("provider: ", Style::default().fg(DIM)),
            provider_chip(Provider::Claude, *provider == Provider::Claude),
            Span::raw("  "),
            provider_chip(Provider::Agents, *provider == Provider::Agents),
        ]),
        Line::from(vec![
            Span::styled("scope:    ", Style::default().fg(DIM)),
            scope_chip("global", *scope == Scope::Global),
            Span::raw("  "),
            scope_chip("project", *scope == Scope::Project),
        ]),
        Line::from(""),
    ];
    lines.push(Line::from(Span::styled(
        "type name · tab provider · ↑/↓ scope · enter create · esc cancel",
        Style::default().fg(DIM),
    )));
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn provider_chip(provider: Provider, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            format!(" {} ", provider.label()),
            Style::default()
                .bg(provider_color(provider))
                .fg(BADGE_FG)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!(" {} ", provider.label()), Style::default().fg(DIM))
    }
}

fn scope_chip(label: &str, active: bool) -> Span<'static> {
    if active {
        Span::styled(
            format!(" {label} "),
            Style::default()
                .bg(ACCENT2)
                .fg(BADGE_FG)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(format!(" {label} "), Style::default().fg(DIM))
    }
}

fn render_install_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::InstallTarget {
        skill_name,
        options,
        checked,
        cursor,
    } = &app.modal
    else {
        return;
    };
    let height = (options.len() as u16) + 8;
    let rect = centered(
        area,
        56.min(area.width.saturating_sub(4)),
        height.min(area.height.saturating_sub(2)),
    );
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT2))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " install ",
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
        ));

    let mut lines = vec![
        Line::from(vec![
            Span::raw("install "),
            Span::styled(
                skill_name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" into:"),
        ]),
        Line::from(""),
    ];
    for (idx, (provider, scope)) in options.iter().enumerate() {
        let on_cursor = *cursor == idx;
        let is_checked = checked.get(idx).copied().unwrap_or(false);
        let mark = if is_checked { "✓" } else { " " };
        let style = if on_cursor {
            Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD)
        } else if is_checked {
            Style::default().fg(ACCENT2)
        } else {
            Style::default().fg(DIM)
        };
        lines.push(Line::from(vec![
            Span::styled(
                if on_cursor { "▸ " } else { "  " },
                Style::default().fg(ACCENT2),
            ),
            Span::styled(format!("[{mark}] {provider} / {scope}"), style),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "j/k move · space select · enter install · esc cancel",
        Style::default().fg(DIM),
    )));
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn render_install_overwrite_modal(f: &mut Frame, app: &App, area: Rect) {
    let Modal::ConfirmInstallOverwrite {
        skill_name,
        provider,
        scope,
        pending,
    } = &app.modal
    else {
        return;
    };
    let extra = if pending.is_empty() { 0 } else { 1 };
    let rect = centered(area, 56.min(area.width.saturating_sub(4)), 9 + extra);
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(WARN))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " overwrite? ",
            Style::default().fg(WARN).add_modifier(Modifier::BOLD),
        ));
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                skill_name.clone(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" already exists in {provider}/{scope}."),
                Style::default().fg(FG),
            ),
        ]),
        Line::from(""),
    ];
    if !pending.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("{} more location(s) queued after this.", pending.len()),
            Style::default().fg(DIM),
        )));
    }
    lines.push(Line::from(Span::styled(
        "y overwrite · n / esc skip",
        Style::default().fg(DIM),
    )));
    f.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false }),
        rect,
    );
}

fn human_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn render_help(f: &mut Frame, area: Rect) {
    let rect = centered(
        area,
        68.min(area.width.saturating_sub(2)),
        30.min(area.height.saturating_sub(2)),
    );
    f.render_widget(Clear, rect);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ACCENT))
        .padding(Padding::uniform(1))
        .title(Span::styled(
            " help — vim keys ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ));

    let rows = [
        ("j / k", "move down / up"),
        ("g g / G", "jump to top / bottom"),
        ("ctrl-d / ctrl-u", "half-page down / up"),
        ("/ then type", "filter · esc clears · n/N cycle"),
        ("tab", "switch between project / global boxes"),
        ("o", "toggle grouping by scope (project / global)"),
        ("enter / l", "open detail · h / esc back"),
        ("a", "create new skill"),
        ("e", "edit SKILL.md body (built-in vim editor)"),
        ("f", "edit frontmatter (name / description)"),
        ("s", "share to another provider (copy | symlink)"),
        ("m", "browse the skills.sh marketplace"),
        ("h", "harness: CLAUDE.md / AGENTS.md (global & project)"),
        ("c", "commands: slash-command files (global & project)"),
        ("x / D", "delete skill (choose instances)"),
        ("r", "reload from disk"),
        ("q", "quit"),
        ("", ""),
        (
            "editor:",
            "i/a/o insert · esc normal · q quit · ctrl+s save",
        ),
        (
            "editor:",
            "dd cut line · yy yank · p paste · u undo · :w :q",
        ),
        ("", ""),
        ("market:", "type + enter search · j/k move · enter view"),
        (
            "market:",
            "i install (space picks locations) · / search · esc/q back",
        ),
        ("", ""),
        (
            "harness:",
            "tab switch box · e edit · s link · x delete · q back",
        ),
        (
            "commands:",
            "tab switch box · e edit · a new · s link · x delete",
        ),
    ];
    let lines: Vec<Line> = rows
        .iter()
        .map(|(k, v)| {
            Line::from(vec![
                Span::styled(
                    format!("{k:<16}"),
                    Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
                ),
                Span::styled((*v).to_string(), Style::default().fg(DIM)),
            ])
        })
        .collect();
    f.render_widget(Paragraph::new(lines).block(block), rect);
}

fn render_status(
    f: &mut Frame,
    app: &App,
    editor: Option<&Editor>,
    market: Option<&Market>,
    area: Rect,
) {
    let content = if app.screen == Screen::Marketplace {
        let m = market;
        let busy = m.map(|m| m.searching || m.fetching).unwrap_or(false);
        if busy {
            let frame = m.map(|m| m.spinner_frame()).unwrap_or(' ');
            let what = if m.map(|m| m.searching).unwrap_or(false) {
                "searching"
            } else {
                "downloading"
            };
            Line::from(Span::styled(
                format!(" {frame} {what}…"),
                Style::default().fg(WARN),
            ))
        } else if let Some((msg, is_error)) = &app.status {
            Line::from(Span::styled(
                format!(" {msg}"),
                Style::default().fg(if *is_error { ERR } else { ACCENT2 }),
            ))
        } else {
            let focus = m.map(|m| m.focus).unwrap_or(MarketFocus::Search);
            let hint = match focus {
                MarketFocus::Search => " type query · enter search · ↓/tab results · esc back",
                MarketFocus::Results => " j/k move · enter view · i install · / search · q back",
                MarketFocus::Detail => " j/k scroll · i install · h back · q exit",
            };
            Line::from(Span::styled(hint, Style::default().fg(DIM)))
        }
    } else if app.screen == Screen::Editor {
        if let Some(ed) = editor {
            let mode = ed.mode;
            let cmd = if mode == VimMode::Command {
                format!(":{}", ed.command)
            } else {
                String::new()
            };
            Line::from(vec![
                Span::styled(
                    format!(" {} ", mode.label()),
                    Style::default()
                        .bg(match mode {
                            VimMode::Normal => ACCENT,
                            VimMode::Insert => ACCENT2,
                            VimMode::Command => WARN,
                        })
                        .fg(BADGE_FG)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(cmd, Style::default().fg(WARN)),
                Span::styled(
                    "  i insert · esc normal · q quit · ^s save · :w :q",
                    Style::default().fg(DIM),
                ),
            ])
        } else {
            Line::from("")
        }
    } else if let Some((msg, is_error)) = &app.status {
        Line::from(Span::styled(
            format!(" {msg}"),
            Style::default().fg(if *is_error { ERR } else { ACCENT2 }),
        ))
    } else {
        let hint = match app.screen {
            Screen::List => {
                " j/k move · / search · a new · e edit · s share · h harness · c commands · x delete · ? help"
            }
            Screen::Detail => " j/k scroll · e edit · f frontmatter · s share · x delete · h back",
            Screen::Harness => {
                " j/k move · tab switch box · e edit · s link claude↔agents · x delete · h/q back"
            }
            Screen::Commands => {
                " j/k move · tab switch box · e edit · a new · s link · x delete · c/q back"
            }
            Screen::Help => " esc close",
            _ => " esc cancel",
        };
        Line::from(Span::styled(hint, Style::default().fg(DIM)))
    };
    f.render_widget(Paragraph::new(content), area);
}

fn app_empty(f: &mut Frame, area: Rect, msg: &str) {
    f.render_widget(
        Paragraph::new(msg)
            .style(Style::default().fg(DIM))
            .alignment(Alignment::Center),
        area,
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w,
        height: h,
    }
}

fn wrap_desc(desc: &str) -> Vec<String> {
    let collapsed = desc.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        vec!["(no description)".to_string()]
    } else {
        vec![collapsed]
    }
}

fn short_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home = home.display().to_string();
        if let Some(stripped) = path.strip_prefix(&home) {
            return format!("~{stripped}");
        }
    }
    path.to_string()
}

fn markdown_text(body: &str) -> Text<'static> {
    let mut lines = Vec::new();
    let mut in_code = false;
    for raw in body.lines() {
        let line = raw.to_string();
        if line.trim_start().starts_with("```") {
            in_code = !in_code;
            lines.push(Line::from(Span::styled(line, Style::default().fg(DIM))));
            continue;
        }
        if in_code {
            lines.push(Line::from(Span::styled(line, Style::default().fg(ACCENT2))));
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("# ") {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("## ") || trimmed.starts_with("### ") {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(ACCENT2).add_modifier(Modifier::BOLD),
            )));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            lines.push(Line::from(Span::styled(line, Style::default().fg(DIM))));
        } else {
            lines.push(Line::from(line));
        }
    }
    Text::from(lines)
}
