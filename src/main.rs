#[macro_use] extern crate rocket;

use dotenv::dotenv;
use crate::api::routes::test;

mod models;
mod utils;
mod api;

#[launch]
fn rocket() -> _ {
    // Load environment variables from .env file
    dotenv().ok();
    
    rocket::build()
        .mount("/", routes![test])
}
