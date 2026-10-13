use super::model::{
    OutputFormat,
    RunMode,
    ScriptSource,
};

/// Database operation. Creating a file and bootstrapping config are not
/// operations: they do not require a connection.
#[derive(Debug)]
pub enum Operation {
    Init,
    New { comment: Option<String>, locked: bool },
    Up(UpOptions),
    Down(DownOptions),
    Apply(ApplyOperation),
    List { output: OutputFormat },
    History(HistoryOperation),
    Diff,
}

impl Operation {
    pub fn checks_store_version(&self) -> bool {
        !matches!(self, |Self::New { .. }| Self::Init)
    }
}

#[derive(Debug)]
pub struct UpOptions {
    pub timeout: Option<u64>,
    pub count: Option<usize>,
    pub preview: bool,
    pub mode: RunMode,
    pub yes: bool,
}

#[derive(Debug)]
pub struct DownOptions {
    pub timeout: Option<u64>,
    pub count: usize,
    pub source: ScriptSource,
    pub preview: bool,
    pub mode: RunMode,
    pub yes: bool,
    pub unlock: bool,
}

#[derive(Debug)]
pub enum ApplyOperation {
    Up {
        id: super::model::MigrationId,
        timeout: Option<u64>,
        mode: RunMode,
        yes: bool,
        locked: bool,
    },
    Down {
        id: super::model::MigrationId,
        timeout: Option<u64>,
        source: ScriptSource,
        mode: RunMode,
        yes: bool,
        unlock: bool,
    },
}

#[derive(Debug)]
pub enum HistoryOperation {
    Sync,
    Fix,
}
