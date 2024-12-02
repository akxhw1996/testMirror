use std::fs::{self, File, OpenOptions};
use std::io::Write;
use env_logger::Builder;
use log::LevelFilter;
use std::path::Path;

pub fn init_production_logger() {
    let log_dir = "logs";
    let log_file = format!("{}/webhook_service.log", log_dir);
    
    // Create logs directory if it doesn't exist
    fs::create_dir_all(log_dir).expect("Failed to create log directory");
    
    // Configure env_logger with custom format
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Info);
    
    // Create or append to log file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .expect("Failed to open log file");
    
    // Set custom format
    builder.format(|buf, record| {
        writeln!(
            buf,
            "{} [{}] {} - {}",
            buf.timestamp(),
            record.level(),
            record.target(),
            record.args()
        )
    });
    
    // Set file as output
    builder.target(env_logger::Target::Pipe(Box::new(file)));
    
    // Initialize the logger
    builder.init();
    
    log::info!("Logger initialized - logging to {}", log_file);
}

#[cfg(test)]
pub fn init_test_logger() {
    let log_dir = "logs/test";
    let log_file = format!("{}/test.log", log_dir);
    
    // Create logs directory if it doesn't exist
    fs::create_dir_all(log_dir).expect("Failed to create test log directory");
    
    // Configure env_logger with custom format
    let mut builder = Builder::new();
    builder.filter_level(LevelFilter::Debug);
    
    // Create or append to log file
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .expect("Failed to open test log file");
    
    // Set custom format
    builder.format(|buf, record| {
        writeln!(
            buf,
            "{} [{}] {} - {}",
            buf.timestamp(),
            record.level(),
            record.target(),
            record.args()
        )
    });
    
    // Set file as output
    builder.target(env_logger::Target::Pipe(Box::new(file)));
    
    // Initialize the logger
    builder.init();
    
    log::info!("Test logger initialized - logging to {}", log_file);
}
