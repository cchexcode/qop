use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub enum MigrationOperation {
    CreateTable {
        name: String,
        columns: Vec<String>,
        if_not_exists: bool,
    },
    DropTable {
        name: String,
    },
    AlterTable {
        name: String,
        action: String,
    },
    CreateIndex {
        name: String,
        table: String,
        columns: Vec<String>,
    },
    DropIndex {
        name: String,
    },
    Insert {
        table: String,
        rows: Option<usize>,
    },
    Update {
        table: String,
    },
    Delete {
        table: String,
    },
    CreateFunction {
        name: String,
    },
    DropFunction {
        name: String,
    },
    Other {
        description: String,
    },
}

pub fn parse_migration_operations(sql: &str) -> Result<Vec<MigrationOperation>> {
    let mut operations = Vec::new();
    
    // Split SQL into statements by semicolon
    let statements: Vec<&str> = sql
        .split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && !s.starts_with("--"))
        .collect();
    
    for statement in statements {
        let statement_upper = statement.to_uppercase();
        let words: Vec<&str> = statement_upper.split_whitespace().collect();
        
        if words.is_empty() {
            continue;
        }
        
        match words[0] {
            "CREATE" => {
                if words.len() > 1 {
                    match words[1] {
                        "TABLE" => {
                            // Handle both "CREATE TABLE name" and "CREATE TABLE IF NOT EXISTS name"
                            let (table_name_index, if_not_exists) = if words.len() > 4 && words[2] == "IF" && words[3] == "NOT" && words[4] == "EXISTS" {
                                (5, true) // CREATE TABLE IF NOT EXISTS name
                            } else {
                                (2, false) // CREATE TABLE name
                            };
                            
                            if let Some(table_name) = extract_table_name(&statement_upper, table_name_index) {
                                let columns = extract_column_definitions(statement);
                                operations.push(MigrationOperation::CreateTable {
                                    name: table_name,
                                    columns,
                                    if_not_exists,
                                });
                            }
                        }
                        "INDEX" | "UNIQUE" => {
                            if let Some((index_name, table_name, columns)) = extract_index_info(&statement_upper) {
                                operations.push(MigrationOperation::CreateIndex {
                                    name: index_name,
                                    table: table_name,
                                    columns,
                                });
                            }
                        }
                        "FUNCTION" | "OR" => {
                            if let Some(func_name) = extract_function_name(&statement_upper) {
                                operations.push(MigrationOperation::CreateFunction {
                                    name: func_name,
                                });
                            }
                        }
                        _ => {
                            operations.push(MigrationOperation::Other {
                                description: format!("CREATE {}", words[1..].join(" ")),
                            });
                        }
                    }
                }
            }
            "DROP" => {
                if words.len() > 1 {
                    match words[1] {
                        "TABLE" => {
                            if let Some(table_name) = extract_table_name(&statement_upper, 2) {
                                operations.push(MigrationOperation::DropTable {
                                    name: table_name,
                                });
                            }
                        }
                        "INDEX" => {
                            if let Some(index_name) = extract_table_name(&statement_upper, 2) {
                                operations.push(MigrationOperation::DropIndex {
                                    name: index_name,
                                });
                            }
                        }
                        "FUNCTION" => {
                            if let Some(func_name) = extract_function_name(&statement_upper) {
                                operations.push(MigrationOperation::DropFunction {
                                    name: func_name,
                                });
                            }
                        }
                        _ => {
                            operations.push(MigrationOperation::Other {
                                description: format!("DROP {}", words[1..].join(" ")),
                            });
                        }
                    }
                }
            }
            "ALTER" => {
                if words.len() > 2 && words[1] == "TABLE" {
                    if let Some(table_name) = extract_table_name(&statement_upper, 2) {
                        let action_raw = words[3..].join(" ");
                        let action = strip_sql_comments(&action_raw).trim().to_string();
                        operations.push(MigrationOperation::AlterTable {
                            name: table_name,
                            action,
                        });
                    }
                }
            }
            "INSERT" => {
                if words.len() > 2 && words[1] == "INTO" {
                    if let Some(table_name) = extract_table_name(&statement_upper, 2) {
                        operations.push(MigrationOperation::Insert {
                            table: table_name,
                            rows: None, // Could be enhanced to count rows
                        });
                    }
                }
            }
            "UPDATE" => {
                if let Some(table_name) = extract_table_name(&statement_upper, 1) {
                    operations.push(MigrationOperation::Update {
                        table: table_name,
                    });
                }
            }
            "DELETE" => {
                if words.len() > 2 && words[1] == "FROM" {
                    if let Some(table_name) = extract_table_name(&statement_upper, 2) {
                        operations.push(MigrationOperation::Delete {
                            table: table_name,
                        });
                    }
                }
            }
            _ => {
                // Catch other SQL operations
                let description = words[0..std::cmp::min(3, words.len())].join(" ");
                if !description.is_empty() {
                    operations.push(MigrationOperation::Other {
                        description,
                    });
                }
            }
        }
    }
    
    if operations.is_empty() {
        return Err(anyhow!("No recognizable SQL operations found"));
    }
    
    Ok(operations)
}

pub fn display_migration_diff(migration_id: &str, operations: &[MigrationOperation]) {
    println!("\n📋 Migration: {}", migration_id);
    
    if operations.is_empty() {
        println!("  No operations detected");
        return;
    }
    
    for operation in operations {
        match operation {
            MigrationOperation::CreateTable { name, columns, if_not_exists } => {
                if *if_not_exists {
                    println!("  ➕ CREATE TABLE IF NOT EXISTS {}", name);
                } else {
                    println!("  ➕ CREATE TABLE {}", name);
                }
                if !columns.is_empty() {
                    for column in columns.iter().take(3) {
                        println!("     • {}", column);
                    }
                    if columns.len() > 3 {
                        println!("     • ... and {} more columns", columns.len() - 3);
                    }
                }
            }
            MigrationOperation::DropTable { name } => {
                println!("  ❌ DROP TABLE {}", name);
            }
            MigrationOperation::AlterTable { name, action } => {
                println!("  🔄 ALTER TABLE {} {}", name, action);
            }
            MigrationOperation::CreateIndex { name, table, columns } => {
                println!("  📊 CREATE INDEX {} ON {} ({})", name, table, columns.join(", "));
            }
            MigrationOperation::DropIndex { name } => {
                println!("  🗑️  DROP INDEX {}", name);
            }
            MigrationOperation::Insert { table, rows } => {
                match rows {
                    Some(count) => println!("  📥 INSERT {} rows into {}", count, table),
                    None => println!("  📥 INSERT into {}", table),
                }
            }
            MigrationOperation::Update { table } => {
                println!("  ✏️  UPDATE {}", table);
            }
            MigrationOperation::Delete { table } => {
                println!("  🗑️  DELETE from {}", table);
            }
            MigrationOperation::CreateFunction { name } => {
                println!("  🔧 CREATE FUNCTION {}", name);
            }
            MigrationOperation::DropFunction { name } => {
                println!("  🗑️  DROP FUNCTION {}", name);
            }
            MigrationOperation::Other { description } => {
                println!("  ⚙️  {}", description);
            }
        }
    }
}

fn extract_table_name(statement: &str, word_index: usize) -> Option<String> {
    let words: Vec<&str> = statement.split_whitespace().collect();
    if words.len() > word_index {
        let name = words[word_index]
            .trim_matches('(')
            .trim_matches('"')
            .trim_matches('`');
        Some(name.to_string())
    } else {
        None
    }
}

fn extract_column_definitions(statement: &str) -> Vec<String> {
    let mut columns = Vec::new();
    
    // Find the opening parenthesis for column definitions
    if let Some(start) = statement.find('(') {
        if let Some(end) = statement.rfind(')') {
            let columns_part = &statement[start + 1..end];
            
            // Split by commas, but be careful of nested parentheses
            let mut current_column = String::new();
            let mut paren_depth = 0;
            
            for ch in columns_part.chars() {
                match ch {
                    '(' => {
                        paren_depth += 1;
                        current_column.push(ch);
                    }
                    ')' => {
                        paren_depth -= 1;
                        current_column.push(ch);
                    }
                    ',' if paren_depth == 0 => {
                        let trimmed = strip_sql_comments(&current_column).trim().to_string();
                        if !trimmed.is_empty() {
                            columns.push(trimmed);
                        }
                        current_column.clear();
                    }
                    _ => {
                        current_column.push(ch);
                    }
                }
            }
            
            // Don't forget the last column
            let trimmed = strip_sql_comments(&current_column).trim().to_string();
            if !trimmed.is_empty() {
                columns.push(trimmed);
            }
        }
    }
    
    columns
}

fn strip_sql_comments(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    
    while let Some(ch) = chars.next() {
        match ch {
            '-' if chars.peek() == Some(&'-') => {
                // Skip line comment (everything until newline or end of string)
                chars.next(); // consume the second '-'
                while let Some(c) = chars.next() {
                    if c == '\n' {
                        result.push(c); // Keep the newline
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                // Skip block comment
                chars.next(); // consume the '*'
                
                // Find the end of block comment
                while let Some(c) = chars.next() {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next(); // consume the '/'
                        break;
                    }
                }
            }
            _ => {
                result.push(ch);
            }
        }
    }
    
    result
}

fn extract_index_info(statement: &str) -> Option<(String, String, Vec<String>)> {
    let words: Vec<&str> = statement.split_whitespace().collect();
    
    // Look for pattern: CREATE [UNIQUE] INDEX index_name ON table_name (columns)
    let mut index_pos = None;
    let mut on_pos = None;
    
    for (i, word) in words.iter().enumerate() {
        if *word == "INDEX" {
            index_pos = Some(i);
        } else if *word == "ON" {
            on_pos = Some(i);
        }
    }
    
    if let (Some(idx_pos), Some(on_pos)) = (index_pos, on_pos) {
        if idx_pos + 1 < words.len() && on_pos + 1 < words.len() {
            let index_name = words[idx_pos + 1].trim_matches('"').trim_matches('`').to_string();
            let table_name = words[on_pos + 1].trim_matches('"').trim_matches('`').to_string();
            
            // Extract columns from parentheses
            let columns = if let Some(start) = statement.find('(') {
                if let Some(end) = statement.rfind(')') {
                    let cols_part = &statement[start + 1..end];
                    cols_part
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').trim_matches('`').to_string())
                        .collect()
                } else {
                    vec![]
                }
            } else {
                vec![]
            };
            
            return Some((index_name, table_name, columns));
        }
    }
    
    None
}

fn extract_function_name(statement: &str) -> Option<String> {
    let words: Vec<&str> = statement.split_whitespace().collect();
    
    // Look for CREATE FUNCTION or CREATE OR REPLACE FUNCTION
    let function_pos = words.iter().position(|&w| w == "FUNCTION")?;
    
    if function_pos + 1 < words.len() {
        let func_name = words[function_pos + 1]
            .split('(')
            .next()?
            .trim_matches('"')
            .trim_matches('`');
        Some(func_name.to_string())
    } else {
        None
    }
}