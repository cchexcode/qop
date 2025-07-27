#[derive(Debug)]
pub enum MigrationApply {
    Up {
        id: String,
        timeout: Option<u64>,
    },
    Down {
        id: String,
        timeout: Option<u64>,
        remote: bool,
    },
}

#[derive(Debug)]
pub enum HistoryCommand {
    Sync,
    Fix,
}

#[derive(Debug)]
pub enum Command {
    Init,
    New,
    Up {
        timeout: Option<u64>,
        count: Option<usize>,
        diff: bool,
    },
    Down {
        timeout: Option<u64>,
        count: Option<usize>,
        remote: bool,
        diff: bool,
    },
    Apply(MigrationApply),
    List,
    History(HistoryCommand),
    Diff,
}