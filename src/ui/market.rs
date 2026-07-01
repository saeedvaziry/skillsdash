use crate::market::{MarketClient, MarketSkill, SkillContent, UreqClient};
use crate::model::{Provider, Scope};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::thread;

pub enum JobResult {
    Search {
        query: String,
        result: Result<Vec<MarketSkill>, String>,
    },
    Fetch {
        skill: MarketSkill,
        result: Result<SkillContent, String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketFocus {
    Search,
    Results,
    Detail,
}

pub struct InstallPrompt {
    pub skill_name: String,
    pub content: SkillContent,
    pub provider: Provider,
    pub scope: Scope,
    pub confirm_overwrite: bool,
}

pub struct Market {
    pub client: Arc<UreqClient>,
    pub query: String,
    pub focus: MarketFocus,
    pub results: Vec<MarketSkill>,
    pub selected: usize,
    pub detail_scroll: u16,
    pub detail: Option<SkillContent>,
    pub searching: bool,
    pub fetching: bool,
    pub last_query: String,
    pub install: Option<InstallPrompt>,
    tx: Sender<JobResult>,
    rx: Receiver<JobResult>,
    spinner: usize,
}

impl Market {
    pub fn new() -> Market {
        let (tx, rx) = std::sync::mpsc::channel();
        Market {
            client: Arc::new(UreqClient::new()),
            query: String::new(),
            focus: MarketFocus::Search,
            results: Vec::new(),
            selected: 0,
            detail_scroll: 0,
            detail: None,
            searching: false,
            fetching: false,
            last_query: String::new(),
            install: None,
            tx,
            rx,
            spinner: 0,
        }
    }

    pub fn selected_skill(&self) -> Option<&MarketSkill> {
        self.results.get(self.selected)
    }

    pub fn detail_for(&self, skill_name: &str) -> Option<&SkillContent> {
        let selected = self.selected_skill()?;
        if selected.name == skill_name {
            self.detail.as_ref()
        } else {
            None
        }
    }

    pub fn spinner_frame(&self) -> char {
        const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
        FRAMES[self.spinner % FRAMES.len()]
    }

    pub fn tick(&mut self) {
        if self.searching || self.fetching {
            self.spinner = self.spinner.wrapping_add(1);
        }
    }

    pub fn start_search(&mut self) {
        let query = self.query.trim().to_string();
        if query.is_empty() {
            return;
        }
        self.searching = true;
        self.last_query = query.clone();
        let client = self.client.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = client.search(&query, 100).map_err(|e| e.to_string());
            let _ = tx.send(JobResult::Search { query, result });
        });
    }

    pub fn start_fetch(&mut self) {
        let Some(skill) = self.selected_skill().cloned() else {
            return;
        };
        self.fetching = true;
        self.detail_scroll = 0;
        let client = self.client.clone();
        let tx = self.tx.clone();
        thread::spawn(move || {
            let result = client.fetch(&skill).map_err(|e| e.to_string());
            let _ = tx.send(JobResult::Fetch { skill, result });
        });
    }

    pub fn poll(&mut self) -> Vec<JobEvent> {
        let mut events = Vec::new();
        while let Ok(job) = self.rx.try_recv() {
            match job {
                JobResult::Search { query, result } => {
                    if query != self.last_query {
                        continue;
                    }
                    self.searching = false;
                    match result {
                        Ok(skills) => {
                            self.results = skills;
                            self.selected = 0;
                            self.detail = None;
                            if self.results.is_empty() {
                                events.push(JobEvent::Info("no results".to_string()));
                            } else {
                                self.focus = MarketFocus::Results;
                            }
                        }
                        Err(e) => events.push(JobEvent::Error(e)),
                    }
                }
                JobResult::Fetch { skill, result } => {
                    self.fetching = false;
                    match result {
                        Ok(content) => {
                            let matches_selection = self
                                .selected_skill()
                                .map(|s| s.id == skill.id)
                                .unwrap_or(false);
                            if matches_selection {
                                self.detail = Some(content);
                                self.focus = MarketFocus::Detail;
                            }
                        }
                        Err(e) => events.push(JobEvent::Error(e)),
                    }
                }
            }
        }
        events
    }

    pub fn move_selection(&mut self, delta: i64) {
        if self.results.is_empty() {
            return;
        }
        let len = self.results.len() as i64;
        let mut next = self.selected as i64 + delta;
        if next < 0 {
            next = 0;
        }
        if next >= len {
            next = len - 1;
        }
        self.selected = next as usize;
    }
}

impl Default for Market {
    fn default() -> Self {
        Market::new()
    }
}

pub enum JobEvent {
    Info(String),
    Error(String),
}
