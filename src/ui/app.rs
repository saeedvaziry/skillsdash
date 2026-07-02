use crate::model::{HarnessFile, HarnessKind, HarnessRegistry, Provider, Registry, Scope, Skill};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillGroup {
    Project,
    Global,
}

impl SkillGroup {
    pub fn of(skill: &Skill) -> SkillGroup {
        if skill.instances.iter().any(|i| i.scope == Scope::Project) {
            SkillGroup::Project
        } else {
            SkillGroup::Global
        }
    }

    pub fn heading(self) -> &'static str {
        match self {
            SkillGroup::Project => "project skills",
            SkillGroup::Global => "global skills",
        }
    }

    fn order(self) -> u8 {
        match self {
            SkillGroup::Global => 0,
            SkillGroup::Project => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    List,
    Detail,
    Editor,
    Form,
    Help,
    Marketplace,
    Harness,
    Commands,
}

impl Screen {
    pub fn harness_kind(self) -> Option<HarnessKind> {
        match self {
            Screen::Harness => Some(HarnessKind::Memory),
            Screen::Commands => Some(HarnessKind::Command),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Modal {
    None,
    ConfirmDelete {
        skill_name: String,
        targets: Vec<(Provider, Scope)>,
        cursor: usize,
    },
    Share {
        skill_name: String,
        options: Vec<(Provider, Scope)>,
        cursor: usize,
        method_choice: Option<usize>,
    },
    Message {
        title: String,
        body: String,
        is_error: bool,
    },
    InstallTarget {
        skill_name: String,
        options: Vec<(Provider, Scope)>,
        checked: Vec<bool>,
        cursor: usize,
    },
    ConfirmInstallOverwrite {
        skill_name: String,
        provider: Provider,
        scope: Scope,
        pending: Vec<(Provider, Scope)>,
    },
    LinkHarness {
        file_index: usize,
        source_label: String,
        target_label: String,
        target_provider: Provider,
    },
    ConfirmDeleteHarness {
        file_index: usize,
        label: String,
        is_symlink: bool,
    },
    CreateCommand {
        name: String,
        provider: Provider,
        scope: Scope,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKind {
    Create,
    EditFrontmatter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormField {
    Name,
    Description,
    Provider,
    Scope,
}

pub struct FormState {
    pub kind: FormKind,
    pub name: String,
    pub description: String,
    pub provider: Provider,
    pub scope: Scope,
    pub field: FormField,
    pub editing_skill: Option<String>,
    pub target_provider: Provider,
    pub target_scope: Scope,
    /// Create-form multi-select. Indexed by `Provider::ALL` / `Scope::ALL`.
    pub providers: [bool; Provider::ALL.len()],
    pub scopes: [bool; Scope::ALL.len()],
    /// Sub-cursor over the chips of the Provider / Scope rows.
    pub provider_cursor: usize,
    pub scope_cursor: usize,
}

impl FormState {
    pub fn selected_providers(&self) -> Vec<Provider> {
        Provider::ALL
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| self.providers[*i])
            .map(|(_, p)| p)
            .collect()
    }

    pub fn selected_scopes(&self) -> Vec<Scope> {
        Scope::ALL
            .iter()
            .copied()
            .enumerate()
            .filter(|(i, _)| self.scopes[*i])
            .map(|(_, s)| s)
            .collect()
    }
}

pub struct App {
    pub registry: Registry,
    pub harness: HarnessRegistry,
    pub harness_selected: usize,
    pub project_dir: PathBuf,
    pub screen: Screen,
    pub prev_screen: Screen,
    pub selected: usize,
    pub search: Option<String>,
    pub search_active: bool,
    pub last_search: String,
    pub detail_scroll: u16,
    pub modal: Modal,
    pub form: Option<FormState>,
    pub status: Option<(String, bool)>,
    pub pending_g: bool,
    pub grouped: bool,
    pub focused_group: SkillGroup,
    pub should_quit: bool,
}

impl App {
    pub fn new(project_dir: PathBuf) -> App {
        let registry = Registry::discover(&project_dir);
        let harness = HarnessRegistry::discover(&project_dir);
        let mut app = App {
            registry,
            harness,
            harness_selected: 0,
            project_dir,
            screen: Screen::List,
            prev_screen: Screen::List,
            selected: 0,
            search: None,
            search_active: false,
            last_search: String::new(),
            detail_scroll: 0,
            modal: Modal::None,
            form: None,
            status: None,
            pending_g: false,
            grouped: true,
            focused_group: SkillGroup::Project,
            should_quit: false,
        };
        app.clamp_selection();
        app.sync_focus_to_selection();
        app
    }

    pub fn filtered_indices(&self) -> Vec<usize> {
        let query = self.search.as_deref().unwrap_or("").to_lowercase();
        let mut indices: Vec<usize> = self
            .registry
            .skills
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                query.is_empty()
                    || s.name.to_lowercase().contains(&query)
                    || s.description.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        if self.grouped {
            // Registry order is already alphabetical, so a stable sort by group
            // keeps names sorted within each group.
            indices.sort_by_key(|&i| SkillGroup::of(&self.registry.skills[i]).order());
        }
        indices
    }

    /// Skills partitioned into their groups, in display order (global first,
    /// then project). Each entry is `(group, rows)` where every row is
    /// `(skill_index, registry_index)`; `skill_index` is the position within
    /// `filtered_indices()`. When grouping is on, BOTH groups are always
    /// returned even if a group is empty, so both boxes render. When grouping
    /// is off, a single `Global`-labeled section holds everything (the label is
    /// ignored by the caller in that case).
    pub fn grouped_sections(&self) -> Vec<(SkillGroup, Vec<(usize, usize)>)> {
        let indices = self.filtered_indices();
        if !self.grouped {
            let rows: Vec<(usize, usize)> = indices
                .iter()
                .enumerate()
                .map(|(skill_index, &registry_index)| (skill_index, registry_index))
                .collect();
            return vec![(SkillGroup::Global, rows)];
        }

        let mut project = Vec::new();
        let mut global = Vec::new();
        for (skill_index, &registry_index) in indices.iter().enumerate() {
            match SkillGroup::of(&self.registry.skills[registry_index]) {
                SkillGroup::Project => project.push((skill_index, registry_index)),
                SkillGroup::Global => global.push((skill_index, registry_index)),
            }
        }
        vec![(SkillGroup::Global, global), (SkillGroup::Project, project)]
    }

    pub fn visible_count(&self) -> usize {
        self.filtered_indices().len()
    }

    /// The group the currently selected skill belongs to, if any.
    pub fn selected_group(&self) -> Option<SkillGroup> {
        self.selected_skill().map(SkillGroup::of)
    }

    /// Rows for one group box, as `(skill_index, registry_index)` pairs.
    fn group_rows(&self, group: SkillGroup) -> Vec<(usize, usize)> {
        self.grouped_sections()
            .into_iter()
            .find(|(g, _)| *g == group)
            .map(|(_, rows)| rows)
            .unwrap_or_default()
    }

    /// Keep `focused_group` pointing at the group the selection sits in, so
    /// that j/k navigation across the group boundary updates which box is
    /// focused. Empty-group focus (set by Tab) is preserved when there is no
    /// selected skill to derive a group from.
    pub fn sync_focus_to_selection(&mut self) {
        if let Some(group) = self.selected_group() {
            self.focused_group = group;
        }
    }

    /// Switch focus to the other group box. If that box has skills, move the
    /// selection to its first skill; if it is empty, focus rests on the empty
    /// box (no row highlighted) so actions like `a` still target it.
    pub fn focus_other_group(&mut self) {
        if !self.grouped {
            return;
        }
        let target = match self.focused_group {
            SkillGroup::Project => SkillGroup::Global,
            SkillGroup::Global => SkillGroup::Project,
        };
        self.focused_group = target;
        let rows = self.group_rows(target);
        if let Some(&(skill_index, _)) = rows.first() {
            self.selected = skill_index;
        }
    }

    pub fn selected_skill(&self) -> Option<&Skill> {
        let indices = self.filtered_indices();
        indices
            .get(self.selected)
            .map(|&i| &self.registry.skills[i])
    }

    pub fn clamp_selection(&mut self) {
        let count = self.visible_count();
        if count == 0 {
            self.selected = 0;
        } else if self.selected >= count {
            self.selected = count - 1;
        }
    }

    pub fn move_selection(&mut self, delta: i64) {
        let count = self.visible_count() as i64;
        if count == 0 {
            return;
        }
        let mut next = self.selected as i64 + delta;
        if next < 0 {
            next = 0;
        }
        if next >= count {
            next = count - 1;
        }
        self.selected = next as usize;
    }

    pub fn select_first(&mut self) {
        self.selected = 0;
    }

    pub fn select_last(&mut self) {
        let count = self.visible_count();
        self.selected = count.saturating_sub(1);
    }

    pub fn reload(&mut self) {
        let name = self.selected_skill().map(|s| s.name.clone());
        self.registry.reload();
        if let Some(name) = name {
            if let Some(pos) = self
                .filtered_indices()
                .iter()
                .position(|&i| self.registry.skills[i].name == name)
            {
                self.selected = pos;
            }
        }
        self.clamp_selection();
    }

    pub fn set_status(&mut self, msg: impl Into<String>, is_error: bool) {
        self.status = Some((msg.into(), is_error));
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    pub fn open_message(
        &mut self,
        title: impl Into<String>,
        body: impl Into<String>,
        is_error: bool,
    ) {
        self.modal = Modal::Message {
            title: title.into(),
            body: body.into(),
            is_error,
        };
    }

    pub fn harness_kind(&self) -> HarnessKind {
        self.screen.harness_kind().unwrap_or(HarnessKind::Memory)
    }

    pub fn harness_view_files(&self, kind: HarnessKind) -> Vec<&HarnessFile> {
        self.harness
            .files
            .iter()
            .filter(|f| f.kind == kind)
            .collect()
    }

    pub fn harness_scope_sections(
        &self,
        kind: HarnessKind,
    ) -> Vec<(Scope, Vec<(usize, &HarnessFile)>)> {
        let mut global = Vec::new();
        let mut project = Vec::new();
        for (pos, file) in self.harness_view_files(kind).into_iter().enumerate() {
            match file.scope {
                Scope::Global => global.push((pos, file)),
                Scope::Project => project.push((pos, file)),
            }
        }
        vec![(Scope::Global, global), (Scope::Project, project)]
    }

    pub fn harness_selected_file(&self) -> Option<&HarnessFile> {
        self.harness_view_files(self.harness_kind())
            .into_iter()
            .nth(self.harness_selected)
    }

    pub fn harness_view_len(&self) -> usize {
        self.harness_view_files(self.harness_kind()).len()
    }

    pub fn harness_move(&mut self, delta: i64) {
        let count = self.harness_view_len() as i64;
        if count == 0 {
            return;
        }
        let mut next = self.harness_selected as i64 + delta;
        if next < 0 {
            next = 0;
        }
        if next >= count {
            next = count - 1;
        }
        self.harness_selected = next as usize;
    }

    pub fn harness_select_last(&mut self) {
        self.harness_selected = self.harness_view_len().saturating_sub(1);
    }

    pub fn harness_focus_other_scope(&mut self) {
        let kind = self.harness_kind();
        let current = self
            .harness_view_files(kind)
            .get(self.harness_selected)
            .map(|f| f.scope);
        let target = match current {
            Some(Scope::Global) => Scope::Project,
            _ => Scope::Global,
        };
        if let Some(pos) = self
            .harness_view_files(kind)
            .iter()
            .position(|f| f.scope == target)
        {
            self.harness_selected = pos;
        }
    }

    pub fn harness_selected_abs_index(&self) -> Option<usize> {
        let kind = self.harness_kind();
        self.harness
            .files
            .iter()
            .enumerate()
            .filter(|(_, f)| f.kind == kind)
            .nth(self.harness_selected)
            .map(|(i, _)| i)
    }

    pub fn reload_harness(&mut self) {
        let kind = self.harness_kind();
        let key = self
            .harness_selected_file()
            .map(|f| (f.provider, f.scope, f.name.clone()));
        self.harness.reload();
        if let Some((provider, scope, name)) = key {
            if let Some(pos) = self
                .harness_view_files(kind)
                .iter()
                .position(|f| f.provider == provider && f.scope == scope && f.name == name)
            {
                self.harness_selected = pos;
            }
        }
        let len = self.harness_view_len();
        if self.harness_selected >= len {
            self.harness_selected = len.saturating_sub(1);
        }
    }
}
