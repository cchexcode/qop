use crate::core::model::ScriptFiles;

pub mod commands;
pub mod config;
pub mod migration;
pub mod repo;

pub(crate) const SCRIPTS: ScriptFiles = ScriptFiles::new("surql", "-- SurrealQL goes here");
