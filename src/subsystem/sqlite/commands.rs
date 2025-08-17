#[derive(Debug)]
pub enum MigrationApply {
    Up {
        id: String,
        timeout: Option<u64>,
        dry: bool,
        yes: bool,
    },
    Down {
        id: String,
        timeout: Option<u64>,
        remote: bool,
        dry: bool,
        yes: bool,
    },
}

#[derive(Debug)]
pub enum HistoryCommand {
    Sync,
    Fix,
}

#[derive(Debug)]
pub enum ConfigCommand {
    Init,
}

#[derive(Debug, Clone, Copy)]
pub enum Output {
    Human,
    Json,
}

#[derive(Debug)]
pub enum Command {
    Init,
    New,
    Up {
        timeout: Option<u64>,
        count: Option<usize>,
        diff: bool,
        dry: bool,
        yes: bool,
    },
    Down {
        timeout: Option<u64>,
        count: Option<usize>,
        remote: bool,
        diff: bool,
        dry: bool,
        yes: bool,
    },
    Apply(MigrationApply),
    List { output: Output },
    History(HistoryCommand),
    Diff,
    Config(ConfigCommand),
}