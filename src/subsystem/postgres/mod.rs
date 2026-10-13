use crate::core::model::ScriptFiles;

pub mod commands;
pub mod config;
pub mod repo;
mod sql;

pub(crate) const SCRIPTS: ScriptFiles = ScriptFiles::new("sql", "-- SQL goes here");
