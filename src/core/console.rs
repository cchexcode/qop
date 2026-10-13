use {
    super::{
        model::{
            Direction,
            MigrationId,
            MigrationStatus,
        },
        outcome::needs_prompt,
    },
    anyhow::Result,
    chrono::{
        Local,
        TimeZone,
    },
    comfy_table::{
        Cell,
        CellAlignment,
        ContentArrangement,
        Table,
        modifiers::UTF8_ROUND_CORNERS,
        presets::UTF8_FULL,
    },
    std::io::{
        self,
        IsTerminal,
        Write,
    },
};

pub struct Console;

impl Console {
    pub fn confirm(message: &str, yes: bool, preview: Option<&dyn Fn() -> Result<()>>) -> Result<bool> {
        if yes {
            return Ok(true);
        }
        if !io::stdin().is_terminal() {
            return Err(needs_prompt(format!(
                "{message} Refusing to prompt because stdin is not a terminal. Pass --yes to continue."
            )));
        }
        loop {
            eprint!("{message} [y/N/d]: ");
            io::stderr().flush()?;
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            match parse_answer(&input) {
                | Answer::Yes => return Ok(true),
                | Answer::No => return Ok(false),
                | Answer::Diff => {
                    eprintln!("\n📋 Migration Details:");
                    if let Some(preview) = preview {
                        preview()?;
                    } else {
                        eprintln!("(no preview available)");
                    }
                    eprintln!();
                },
                | Answer::Invalid => eprintln!("Please enter 'y' (yes), 'n' (no), or 'd' (diff)"),
            }
        }
    }

    pub fn warn_out_of_order(ids: &[&MigrationId], latest: &MigrationId) {
        eprintln!("⚠️  Non-linear history detected!");
        eprintln!("The following migrations would create a non-linear history:");
        for id in ids {
            eprintln!("  - {id}");
        }
        eprintln!("Latest applied migration: {latest}");
        eprintln!();
        eprintln!("This could cause issues with database schema consistency.");
        eprintln!("Alternatively, you can run history fix to rename out-of-order migrations.");
    }

    pub fn write_script(out: &mut dyn Write, id: &MigrationId, sql: &str, direction: Direction) -> Result<()> {
        let header = "────────────────────────────────────────────────────────";
        writeln!(out)?;
        writeln!(out, "▶ Migration: {id} [{}]", direction.label())?;
        writeln!(out, "{header}")?;
        write!(out, "{sql}")?;
        if !sql.ends_with('\n') {
            writeln!(out)?;
        }
        writeln!(out, "{header}")?;
        writeln!(out)?;
        Ok(())
    }

    pub fn preview_script(id: &MigrationId, sql: &str, direction: Direction) -> Result<()> {
        Self::write_script(&mut io::stderr(), id, sql, direction)
    }

    pub fn print_script(id: &MigrationId, sql: &str, direction: Direction) -> Result<()> {
        Self::write_script(&mut io::stdout(), id, sql, direction)
    }

    pub fn render_table(rows: &[MigrationStatus]) -> Result<()> {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Migration ID"),
                Cell::new("Remote"),
                Cell::new("Local"),
                Cell::new("Comment"),
                Cell::new("Locked"),
            ]);
        for row in rows {
            let remote = if let Some(applied_at) = row.applied_at {
                Local
                    .from_utc_datetime(&applied_at)
                    .format("%Y-%m-%d %H:%M:%S %Z")
                    .to_string()
            } else {
                "❌".to_string()
            };
            let local = if row.local { "✅" } else { "❌" };
            let comment = row.comment.clone().unwrap_or_else(|| "-".to_string());
            let locked = if row.locked { "🔒" } else { "" };
            table.add_row(vec![
                Cell::new(row.id.to_string()),
                Cell::new(remote).set_alignment(CellAlignment::Center),
                Cell::new(local).set_alignment(CellAlignment::Center),
                Cell::new(comment),
                Cell::new(locked).set_alignment(CellAlignment::Center),
            ]);
        }
        println!("{table}");
        Ok(())
    }

    pub fn report(count: usize, verb: &str, mode: super::model::RunMode) {
        if count == 0 {
            return;
        }
        match mode {
            | super::model::RunMode::Apply => println!("\n🎉 Successfully {verb} {count} migration(s)!"),
            | super::model::RunMode::DryRun => {
                println!("\n🎉 Executed {count} migration(s) in dry-run mode (rolled back).")
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Answer {
    Yes,
    No,
    Diff,
    Invalid,
}

fn parse_answer(input: &str) -> Answer {
    match input.trim().to_ascii_lowercase().as_str() {
        | "y" | "yes" => Answer::Yes,
        | "n" | "no" | "" => Answer::No,
        | "d" | "diff" => Answer::Diff,
        | _ => Answer::Invalid,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_confirmation_answers() {
        assert_eq!(parse_answer(" Y\n"), Answer::Yes);
        assert_eq!(parse_answer(""), Answer::No);
        assert_eq!(parse_answer("diff"), Answer::Diff);
        assert_eq!(parse_answer("maybe"), Answer::Invalid);
    }
}
