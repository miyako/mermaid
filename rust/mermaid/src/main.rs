use axum::{
    extract::{State},
    http::{StatusCode, header},
    response::{Response, IntoResponse},
    routing::{post},
    Json, Router,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::{
    compression::CompressionLayer,
    cors::{CorsLayer, Any},
    trace::TraceLayer,
};
use clap::Parser;
use mermaid_rs::Mermaid;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;

use tracing::{error, info};
use tracing_subscriber::{EnvFilter};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

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
    
    #[arg(short, long, default_value_t = 8080)]
    port: u16,
    
    #[arg(long, default_value_t = false)]
    server: bool,
}

#[derive(Clone)]
struct AppState {
    mermaid: Arc<Mermaid>,  
}

fn write_output(cli: &Cli, json: &String) -> anyhow::Result<()> {
        match &cli.output {
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    
    let mermaid: Arc<Mermaid> = Arc::new(Mermaid::new().unwrap());
    let state = AppState { mermaid: mermaid.clone() };

    if cli.server {
        
        let port = cli.port;
        
        // Set up tracing/logging
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
            .init();
            
        // Build router
        let app = Router::new()
            .route("/render", post(post_render))
            .with_state(state.clone())
            // Middlewares
            .layer(TraceLayer::new_for_http())
            .layer(CompressionLayer::new())
            .layer(
                CorsLayer::new()
                    .allow_methods(Any)
                    .allow_origin(Any)
                    .allow_headers(Any),
            );
        
        // Server address
        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        info!("Starting server on {}", addr);
        
        // Graceful shutdown signal (Ctrl+C)
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        let graceful = async {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Shutdown signal received (Ctrl+C)");
                }
                _ = shutdown_rx => {
                    info!("Shutdown triggered programmatically");
                }
            }
        };
        
        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(graceful);
        
        if let Err(err) = server.await {
            error!("Server error: {}", err);
        }
        
        let _ = shutdown_tx.send(());
        
    }else {

        // Read Markdown content from file or stdin
        let text = match &cli.input {
            Some(path) => fs::read_to_string(path)?,
            None => {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            }
        };
        
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
        
        let _ = write_output(&cli, &json);
                                
    }

    Ok(())
}

async fn post_render(
    State(state): State<AppState>,
    text: String,
) -> impl IntoResponse {

    let mermaid = state.mermaid;    

    let diagram;
    
    match mermaid.render(&text) {
        Ok(svg) => diagram = svg,
        Err(_) => diagram = String::new(),
    }

    if diagram == "" {
        let err = ErrorResponse {
            message: "render failed".to_string(),
        }; 
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();      
    }

    return Response::builder()
       .status(StatusCode::OK)
       .header(header::CONTENT_TYPE, "image/svg+xml")
       .body(diagram).unwrap()
       .into_response();
  
}
