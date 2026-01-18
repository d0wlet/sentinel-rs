mod config;
mod parser;
mod state;

use crate::config::load_config;
use crate::parser::LogParser;
use crate::state::AppState;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Sparkline},
};
use std::io;
use std::sync::Arc;
use std::time::Duration;
use tokio::fs::File;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Setup
    let args: Vec<String> = std::env::args().collect();
    let is_simulator = args.contains(&"--simulate".to_string());
    let log_path = "test.log";

    let config = load_config("config.yaml")?;
    let state = Arc::new(AppState::new(config.webhook_url.clone()));
    let parser = Arc::new(LogParser::new(&config.rules));

    // 2. Spawn Log Processor
    let state_clone = state.clone();
    let parser_clone = parser.clone();
    let path_clone = log_path.to_string();

    // START SIMULATOR IF REQUESTED
    if is_simulator {
        tokio::spawn(async move {
            // Internal generator logic
            use tokio::io::AsyncWriteExt;
            let mut file = File::create(&path_clone).await.unwrap();
            let mut counter = 0;
            loop {
                counter += 1;
                let log = if counter % 500 == 0 {
                    format!("panic!: Kernel panic at main.rs:{}\n", counter)
                } else if counter % 700 == 0 {
                    format!(
                        "{{\"level\": \"error\", \"msg\": \"Critical usage {}\"}}\n",
                        counter
                    )
                } else {
                    format!("[INFO] System healthy {}\n", counter)
                };

                let _ = file.write_all(log.as_bytes()).await;
                // High speed write
                if counter % 100 == 0 {
                    file.flush().await.unwrap();
                    sleep(Duration::from_millis(1)).await;
                }
            }
        });
    } else {
        // Ensure file exists for linemux if not simulating (linemux might error if missing)
        if tokio::fs::metadata(log_path).await.is_err() {
            File::create(log_path).await?;
        }
    }

    // TAIL LOGIC (Linemux)
    tokio::spawn(async move {
        // We use linemux to handle rotation and standardized tailing
        let mut lines = linemux::MuxedLines::new().expect("Could not initialize linemux");
        lines
            .add_file(log_path)
            .await
            .expect("Failed to add file to tail");

        while let Ok(Some(line)) = lines.next_line().await {
            parser_clone.process_line(line.line(), &state_clone);
        }
    });

    // 3. TUI (The "Consumer")
    // Runs on the main thread
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal, state);

    // 4. Cleanup
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: Arc<AppState>,
) -> io::Result<()> {
    // Local history buffer for Sparkline (UI Thread Only)
    let mut error_history: Vec<u64> = vec![0; 100];
    let mut last_total_errors = state
        .total_errors
        .load(std::sync::atomic::Ordering::Relaxed);
    let mut last_update = std::time::Instant::now();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(40),
                        Constraint::Percentage(30),
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Update buckets for time progression (UI Side)
            if last_update.elapsed() >= Duration::from_millis(1000) {
                let current_total = state
                    .total_errors
                    .load(std::sync::atomic::Ordering::Relaxed);
                let delta = current_total.saturating_sub(last_total_errors);

                error_history.remove(0);
                error_history.push(delta);

                last_total_errors = current_total;
                last_update = std::time::Instant::now();
            }

            // 1. Stats Block
            let total_lines = state.total_lines.load(std::sync::atomic::Ordering::Relaxed);
            let total_errors = state
                .total_errors
                .load(std::sync::atomic::Ordering::Relaxed);
            let elapsed = state.start_time.elapsed().as_secs();
            let rate = if elapsed > 0 {
                total_lines / elapsed
            } else {
                0
            };

            let stats_text = format!(
                "Lines Processed: {}\nErrors Found: {}\nTime Elapsed: {}s\nRate: {} lines/s",
                total_lines, total_errors, elapsed, rate
            );

            let stats_paragraph = Paragraph::new(stats_text).block(
                Block::default()
                    .title("Sentinel Status")
                    .borders(Borders::ALL),
            );
            f.render_widget(stats_paragraph, chunks[0]);

            // 2. Visual "Sparkline" (Error Rate History)
            let sparkline = Sparkline::default()
                .block(
                    Block::default()
                        .title("Error Rate (Last 100s)")
                        .borders(Borders::ALL),
                )
                .data(&error_history)
                .style(Style::default().fg(Color::Red));
            f.render_widget(sparkline, chunks[1]);

            // 3. Last Alert
            let last_err = state.last_error.lock().unwrap();
            let alert_text = last_err
                .clone()
                .unwrap_or_else(|| "No errors yet.".to_string());
            let alert_widget = Paragraph::new(alert_text)
                .style(Style::default().fg(if last_err.is_some() {
                    Color::Red
                } else {
                    Color::Gray
                }))
                .block(Block::default().title("Last Alert").borders(Borders::ALL));
            f.render_widget(alert_widget, chunks[2]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    return Ok(());
                }
            }
        }
    }
}
