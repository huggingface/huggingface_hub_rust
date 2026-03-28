use anyhow::Result;
use comfy_table::{Cell, Table};
use serde_json::Value;

use crate::cli::OutputFormat;

#[allow(dead_code)]
pub enum CommandResult {
    Formatted {
        output: CommandOutput,
        format: OutputFormat,
        quiet: bool,
    },
    Raw(String),
    Silent,
}

pub struct CommandOutput {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub json_value: Value,
    pub quiet_values: Vec<String>,
}

impl CommandOutput {
    #[allow(dead_code)]
    pub fn single_item(json_value: Value) -> Self {
        let (headers, rows) = if let Value::Object(ref map) = json_value {
            let headers = vec!["Key".to_string(), "Value".to_string()];
            let rows = map
                .iter()
                .map(|(k, v)| {
                    let display = match v {
                        Value::String(s) => s.clone(),
                        Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    vec![k.clone(), display]
                })
                .collect();
            (headers, rows)
        } else {
            (vec![], vec![vec![json_value.to_string()]])
        };

        CommandOutput {
            headers,
            rows,
            json_value,
            quiet_values: vec![],
        }
    }
}

pub fn render(result: CommandResult) -> Result<()> {
    match result {
        CommandResult::Silent => {},
        CommandResult::Raw(s) => println!("{s}"),
        CommandResult::Formatted { output, format, quiet } => {
            if quiet {
                for val in &output.quiet_values {
                    println!("{val}");
                }
                return Ok(());
            }
            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&output.json_value)?);
                },
                OutputFormat::Table => {
                    let mut table = Table::new();
                    if !output.headers.is_empty() {
                        table.set_header(output.headers.iter().map(Cell::new));
                    }
                    for row in &output.rows {
                        table.add_row(row.iter().map(Cell::new));
                    }
                    println!("{table}");
                },
            }
        },
    }
    Ok(())
}
