#[macro_use] extern crate rocket;

use rocket::routes;
use std::sync::RwLock;
use std::process;
use crate::api::routes::{github_handle, gitcode_handle};
use std::env;
use hex::decode;
use crate::utils::aes_cbc;
use log::{info, error};
use keyring::Entry;

mod models;
mod utils;
mod api;

const SERVICE_NAME: &str = "webhook_service";
const USERNAME: &str = "webhook";
pub fn get_service_key() -> Result<String, keyring::Error> {
    let entry = Entry::new(SERVICE_NAME, USERNAME)?;
    match entry.get_password() {
        Ok(password) => {
            info!("Service key retrieved from keyring");
            Ok(password)
        }
        Err(err) => {
            error!("Failed to retrieve service key from keyring: {}", err);
            Err(err)
        }
    }
}

#[launch]
fn rocket() -> _ {
    // Initialize logger
    utils::logging::init_production_logger();
    info!("Starting webhook service...");

    // Load environment variables from .env file
    dotenv::dotenv().ok();
    
    // Get service key
    let password = match get_service_key() {
        Ok(password) => password,
        Err(err) => {
            error!("Failed to retrieve service key: {}", err);
            process::exit(1);
        }
    };
    let key = utils::hash::sha256_hex(&password);
    
    // Decrypt environment variables
    let env_vars = [
        "GITCODE_TOKEN_ENCRYPTED",
        "GITCODE_WEBHOOK_VERIFYING_KEY_ENCRYPTED",
        "GITHUB_TOKEN_ENCRYPTED",
        "GITHUB_WEBHOOK_VERIFYING_KEY_ENCRYPTED"
    ];
    
    for var_name in env_vars.iter() {
        if let Ok(encrypted_value) = env::var(var_name) {
            let encrypted_bytes = decode(&encrypted_value).unwrap_or_else(|_| {
                error!("Failed to decode hex value for {}", var_name);
                process::exit(1);
            });
            
            let key_bytes = hex::decode(&key).unwrap_or_else(|_| {
                error!("Failed to decode hex key");
                process::exit(1);
            });
            let decrypted_bytes = aes_cbc::decrypt(&key_bytes, &encrypted_bytes).unwrap_or_else(|err| {
                error!("Failed to decrypt {}: {}", var_name, err);
                process::exit(1);
            });
            
            let decrypted_value = String::from_utf8(decrypted_bytes).unwrap_or_else(|_| {
                error!("Failed to convert decrypted bytes to UTF-8 string for {}", var_name);
                process::exit(1);
            });
            
            let env_var_name = var_name.replace("_ENCRYPTED", "");
            env::set_var(&env_var_name, &decrypted_value);
            info!("Successfully decrypted and set {}", env_var_name);
        } else {
            error!("Environment variable {} not found", var_name);
            process::exit(1);
        }
    }
    
    info!("Environment variables decrypted successfully");
    info!("Configuring Rocket server...");

    rocket::build()
        .mount("/", routes![github_handle, gitcode_handle])
        .manage(RwLock::new(true))
}
