#[macro_use] extern crate rocket;

use rocket::routes;
use std::sync::RwLock;
use std::process;
use rpassword::read_password;
use std::io::Write;
use crate::api::routes::{github_handle, gitcode_handle};
use std::env;
use hex::decode;
use crate::utils::aes_cbc;

mod models;
mod utils;
mod api;


#[launch]
fn rocket() -> _ {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    print!("Enter service key: ");
    std::io::stdout().flush().unwrap();
    let password = read_password().unwrap();
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
                eprintln!("Failed to decode hex value for {}", var_name);
                process::exit(1);
            });
            
            let key_bytes = hex::decode(&key).unwrap_or_else(|_| {
                eprintln!("Failed to decode hex key");
                process::exit(1);
            });
            let decrypted_bytes = aes_cbc::decrypt(&key_bytes, &encrypted_bytes).unwrap_or_else(|err| {
                eprintln!("Failed to decrypt {}: {}", var_name, err);
                process::exit(1);
            });
            
            let decrypted_value = String::from_utf8(decrypted_bytes).unwrap_or_else(|_| {
                eprintln!("Failed to convert decrypted bytes to string for {}", var_name);
                process::exit(1);
            });

            
            // Set decrypted value to environment variable without _ENCRYPTED suffix
            let new_var_name = var_name.replace("_ENCRYPTED", "");

            println!("Decrypted value for {}: {}", new_var_name, decrypted_value);
            env::set_var(&new_var_name, decrypted_value);
        }
    }
    
    rocket::build()
        .mount("/", routes![github_handle, gitcode_handle])
        .manage(RwLock::new(String::new()))
}
