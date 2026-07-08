use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub score: i64,
    pub plugin_id: &'static str,
    pub metadata: HashMap<String, String>,
}

pub struct Context {
    pub exit_requested: bool,
    pub message: Option<String>,
    pub command_to_run: Option<(String, Vec<String>, bool)>, // (cmd, args, run_in_terminal)
    pub editor: String,
    pub shell: String,
}

impl Context {
    pub fn new(editor: String, shell: String) -> Self {
        Self {
            exit_requested: false,
            message: None,
            command_to_run: None,
            editor,
            shell,
        }
    }

    pub fn exit(&mut self) {
        self.exit_requested = true;
    }

    pub fn show_message(&mut self, msg: String) {
        self.message = Some(msg);
    }

    pub fn run_command(&mut self, cmd: String, args: Vec<String>, in_terminal: bool) {
        self.command_to_run = Some((cmd, args, in_terminal));
    }
}

pub enum ExecutionResult {
    Success,
    Exit,
    HideAndRun(String, Vec<String>, bool),
    Message(String),
}

pub trait Plugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    // Search matches based on the user's query
    fn search(&self, query: &str, cache_dir: &Path) -> Vec<SearchResult>;

    // Generate markdown or text preview for the selected result
    fn preview(&self, _item: &SearchResult) -> Option<String> {
        None
    }

    // Run action for selected search result
    fn execute(&self, item: &SearchResult, ctx: &mut Context) -> ExecutionResult;
}
