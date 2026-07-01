use crate::model::{Provider, Registry, Scope, Skill};
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
            SkillGroup::Project => 0,
            SkillGroup::Global => 1,
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
        cursor: usize,
    },
    ConfirmInstallOverwrite {
        skill_name: String,
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
}

pub struct App {
    pub registry: Registry,
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
    pub should_quit: bool,
}

impl App {
    pub fn new(project_dir: PathBuf) -> App {
        let registry = Registry::discover(&project_dir);
        let mut app = App {
            registry,
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
            should_quit: false,
        };
        app.clamp_selection();
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

    /// Skills partitioned into their groups, in display order (project first).
    /// Each entry is `(group, rows)` where every row is `(skill_index,
    /// registry_index)`; `skill_index` is the position within
    /// `filtered_indices()`. Only groups that contain at least one skill are
    /// returned. When grouping is off, a single `Global`-labeled section holds
    /// everything (the label is ignored by the caller in that case).
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

        let mut sections: Vec<(SkillGroup, Vec<(usize, usize)>)> = Vec::new();
        for (skill_index, &registry_index) in indices.iter().enumerate() {
            let group = SkillGroup::of(&self.registry.skills[registry_index]);
            match sections.last_mut() {
                Some((g, rows)) if *g == group => rows.push((skill_index, registry_index)),
                _ => sections.push((group, vec![(skill_index, registry_index)])),
            }
        }
        sections
    }

    pub fn visible_count(&self) -> usize {
        self.filtered_indices().len()
    }

    /// The group the currently selected skill belongs to, if any.
    pub fn selected_group(&self) -> Option<SkillGroup> {
        self.selected_skill().map(SkillGroup::of)
    }

    /// Move the selection into the first skill of the other group box.
    /// No-op unless grouping is on and both groups are present.
    pub fn focus_other_group(&mut self) {
        if !self.grouped {
            return;
        }
        let sections = self.grouped_sections();
        if sections.len() < 2 {
            return;
        }
        let current = self.selected_group();
        // Pick the first section whose group differs from the current one.
        if let Some((_, rows)) = sections
            .iter()
            .find(|(group, _)| Some(*group) != current)
            .filter(|(_, rows)| !rows.is_empty())
        {
            self.selected = rows[0].0;
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
}
