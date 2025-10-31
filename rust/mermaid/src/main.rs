use clap::Parser;
use mermaid_rs::Mermaid;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
// use serde::{Serialize, Deserialize};

/// CLI to convert Mermaid diagrams in Markdown to SVG
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Input Markdown file (stdin if omitted)
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Output SVG file (stdout if omitted)
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Input is JSON (default: false)
    #[arg(long, default_value_t = false)]
    batch: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Read Markdown content from file or stdin
    let text = match &cli.input {
        Some(path) => fs::read_to_string(path)?,
        None => {
            let mut buffer = String::new();
            io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };
    
    let mermaid = Mermaid::new().unwrap();
    let json : String;
     
    if cli.batch {
        let mut diagrams: Vec<String> = vec![];  
        let mermaids: Vec<String> = serde_json::from_str(&text)?;
        for (_, md) in mermaids.iter().enumerate() {
            match mermaid.render(&md) {
                Ok(svg) => diagrams.push(svg),
                Err(_) => diagrams.push(String::new()),
            }
        }
        json = serde_json::to_string_pretty(&diagrams)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;            
    } else {
        match mermaid.render(&text) {
            Ok(svg) => json = svg,
            Err(_) => json = String::new(),
        }
    } 
        
    match cli.output {
        Some(path) => {
            let mut f = File::create(path)?;
            f.write_all(json.as_bytes())?;
        }
        None => {
            let mut out = io::stdout();
            out.write_all(json.as_bytes())?;
        }
    }

    Ok(())
}
