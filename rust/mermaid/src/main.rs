use axum::{
    // extract::{State},
    body::{boxed, Full},
    http::{StatusCode, header},
    response::{Response, IntoResponse},
    routing::{post},
    Json, Router,
};
use axum::extract::State;
use std::sync::Arc;
use std::{net::SocketAddr/*, sync::Arc*/};
use tower_http::{
    compression::CompressionLayer,
    cors::{CorsLayer, Any},
    trace::TraceLayer,
};
use headless_chrome::protocol::cdp::Page::Viewport;
use headless_chrome::{Browser, protocol::cdp::Page::CaptureScreenshotFormatOption}; 
use clap::Parser;
use mermaid_rs::Mermaid;
use std::fs;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter};
use serde::Serialize;
use escape_string::escape;
use unescape::unescape;
use std::sync::Mutex;

#[derive(Debug, Serialize)]
struct ErrorResponse {
    message: String,
}

#[derive(serde::Deserialize)]
struct RenderRequest {
    text: String,
    format: Option<String>,
    // x: Option<f64>,
    // y: Option<f64>,
    // width: Option<f64>,
    // height: Option<f64>,
    scale: Option<f64>,
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

// #[derive(Clone)]
struct AppState {
    browser: Arc<Mutex<Browser>>,  
    mermaid_js: Arc<&'static str>,
    html_payload: Arc<&'static str>,
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
    
    let browser: Arc<Mutex<Browser>> = Arc::new(Mutex::new(Browser::default()?));
    // let browser: Arc<Browser> = Browser::default()?.into();   
    let _mermaid_js: &'static str = include_str!("../payload/mermaid.min.js");
    let mermaid_js: Arc<&'static str> = Arc::new(_mermaid_js);
    let _html_payload: &'static str = include_str!("../payload/index.html");
    let html_payload: Arc<&'static str> = Arc::new(_html_payload);
     
    let state = AppState { 
        browser: browser, 
        mermaid_js: mermaid_js,
        html_payload: html_payload,
     };

    if cli.server {
        
        let port = cli.port;
        
        // Set up tracing/logging
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
            .init();
            
        // Build router
        let app = Router::new()
            .route("/render", post(post_render))
            .with_state(state.into())
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
        let mermaid = Mermaid::new().unwrap();
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
    State(state): State<Arc<AppState>>,
    Json(payload): Json<RenderRequest>,
) -> impl IntoResponse {

    // let mermaid = state.mermaid;  
    let text = payload.text;  
    let format = payload.format.unwrap_or("svg".to_string());  
    let x = 0.0;
    let y = 0.0;
    let width;// = payload.width.unwrap_or(2048.0);
    let height;// = payload.height.unwrap_or(2048.0);
    let scale = payload.scale.unwrap_or(1.0);
    
    // let browser = &state.browser;
    let mermaid_js = &state.mermaid_js;
    // let mermaid_js = include_str!("../payload/mermaid.min.js");
    // let html_payload = include_str!("../payload/index.html");
    let html_payload = &state.html_payload;
    
    /*
    let browser: Arc<Mutex<Browser>> = Arc::new(Mutex::new(Browser::default()?));
    */
    
    let tab = {
        // Lock only while creating a new tab
        let mut browser_guard = state.browser.lock().unwrap();
        match browser_guard.new_tab() {
            Ok(tab) => tab,
            Err(_) => {
                
                eprintln!("⚠️ Browser tab creation failed, recreating browser...");

                // Try to create a new browser instance
                match Browser::default() {
                    Ok(new_browser) => {
                        // Replace the old browser in state
                        *browser_guard = new_browser;
                
                        // Try again
                        match browser_guard.new_tab() {
                            Ok(tab) => tab,
                            Err(_) => {
                                let err = ErrorResponse {
                                    message: "Failed to create tab even after browser restart".into(),
                                };
                                return (StatusCode::BAD_REQUEST, Json(err)).into_response();
                            }
                        }
                    }
                    Err(_) => {
                        let err = ErrorResponse {
                            message: "Failed to restart browser".into(),
                        };
                        return (StatusCode::BAD_REQUEST, Json(err)).into_response();
                    }
                }
            }
        }
    }; // <- mutex guard dropped here
    
    /*
    let tab = match browser.new_tab() {
        Ok(t) => t,
        Err(_) => {
            let err = ErrorResponse {
                message: "failed to open tab for diagram".to_string(),
            }; 
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
        }
    };
    */
    let data_url_html = format!("data:text/html;charset=utf-8,{}", html_payload);
    
    if let Err(_) = tab.navigate_to(&data_url_html) {
        let _guard = scopeguard::guard(tab, |t| {
            let _ = t.close(false);
        });
        let err = ErrorResponse {
            message: "failed to navigate to tab for diagram".to_string(),
        }; 
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
    }
    
    // let tab = browser.new_tab()?;
    // tab.evaluate(mermaid_js, false)?;
    // let mermaid = Mermaid::new().unwrap();
    
    let diagram;
    
    /*
    match mermaid.render(&text) {
        Ok(svg) => diagram = svg,
        Err(_) => diagram = String::new(),
    }
    */
    
    /*
    let tab = match browser.new_tab() {
        Ok(t) => t,
        Err(_) => {
            let err = ErrorResponse {
                message: "failed to open tab".to_string(),
            }; 
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
        }
    };
    */
    
    if let Err(_) = tab.evaluate(mermaid_js, false) {
        let _guard = scopeguard::guard(tab, |t| {
            let _ = t.close(false);
        });
        let err = ErrorResponse {
            message: "failed to wait until navigated to tab for diagram".to_string(),
        }; 
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
    }
    
    let data = match tab.evaluate(&format!("render('{}')", escape(&text)), true) {
        Ok(t) => t,
        Err(_) => {
            let _guard = scopeguard::guard(tab, |t| {
                let _ = t.close(false);
            });
            let err = ErrorResponse {
                message: "failed to evaluate diagram".to_string(),
            }; 
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
        }
    };
    
    let string = data.value.unwrap_or_default().to_string();
    let slice = unescape(string.trim_matches('"')).unwrap_or_default();
    
    if slice == "null" {
        diagram = String::new();
    } else {
        diagram = slice.to_string();
    }
    
    if diagram == "" {
        let _guard = scopeguard::guard(tab, |t| {
            let _ = t.close(false);
        });
        let err = ErrorResponse {
            message: "render failed".to_string(),
        }; 
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();      
    }
    
    if format == "png" {

    let data_url = format!("data:image/svg+xml,{}", urlencoding::encode(&diagram));
    
    if let Err(_) = tab.navigate_to(&data_url) {
        let _guard = scopeguard::guard(tab, |t| {
            let _ = t.close(false);
        });
        let err = ErrorResponse {
            message: "failed to navigate to tab for screenshot".to_string(),
        }; 
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
    }
    
    if let Err(_) = tab.wait_until_navigated() {
        let _guard = scopeguard::guard(tab, |t| {
            let _ = t.close(false);
        });
        let err = ErrorResponse {
            message: "failed to wait until navigated to tab for screenshot".to_string(),
        }; 
        return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
    }
        
    let metrics = match tab.evaluate(
        "({
            width: document.documentElement.scrollWidth,
            height: document.documentElement.scrollHeight
        })",
        false,
    ) {
        Ok(metrics) => metrics,
        Err(_) => {
            let _guard = scopeguard::guard(tab, |t| {
                let _ = t.close(false);
            });
            let err = ErrorResponse {
                message: "failed to resize viewport for screenshot".to_string(),
            }; 
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
        }
    };
    
    let size = metrics
    .value
    .unwrap()
    .as_object()
    .unwrap()
    .clone();
    
    width = (size["width"].as_f64().unwrap() as u32).into();
    height = (size["height"].as_f64().unwrap() as u32).into();
    
    let viewport = Viewport {
        x: x,            // left offset
        y: y,            // top offset
        width: width,     // width of capture area in pixels
        height: height,     // height of capture area in pixels
        scale: scale,        // scaling factor (1.0 = 1:1, >1 = higher DPI)
    };
    
    let png_data = match tab.capture_screenshot(CaptureScreenshotFormatOption::Png, None, Some(viewport), true) {
        Ok(data) => data,
        Err(_) => {
            let _guard = scopeguard::guard(tab, |t| {
                let _ = t.close(false);
            });
            let err = ErrorResponse {
                message: "failed to capture screenshot".to_string(),
            }; 
            return (StatusCode::BAD_REQUEST, Json(err)).into_response();  
        }
    };
    
    let _guard = scopeguard::guard(tab, |t| {
        let _ = t.close(false);
    });
    return Response::builder()
       .status(StatusCode::OK)
       .header(header::CONTENT_TYPE, "image/png")
       .body(boxed(Full::from(png_data))) 
       .unwrap()
       .into_response();        
        
    }else{
        
        let _guard = scopeguard::guard(tab, |t| {
            let _ = t.close(false);
        });
        return Response::builder()
           .status(StatusCode::OK)
           .header(header::CONTENT_TYPE, "image/svg+xml")
           .body(diagram).unwrap()
           .into_response();
    }  
}
