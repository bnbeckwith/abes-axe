// Setup Rocket
#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate abes_axe;
extern crate clap;
extern crate rocket;

use std::collections::{HashMap};
use std::sync::{Mutex, Arc};
use abes_axe::{Axe};
use rocket::{State};

type AxeMap = HashMap<String,Axe>;
type AxeMapRef = Arc<Mutex<AxeMap>>;

#[get("/")]
fn index(axeset: State<AxeMapRef>) -> String {
    let axes = axeset.lock().unwrap();
    format!("Set size: {}", axes.len())
}

// #[post("/add_repo", data="<path>")]
// fn add_repository(path: &str, axeset: State<AxeMapRef>) -> String {
//     let axes = axeset.lock().unwrap();
//     format!("")
//}

fn main() {
    let axe_collection: HashMap<String, Axe> = HashMap::new();
    
    // Run server here with axe in place....
    rocket::ignite()
        .mount("/", routes![index])
        .manage(Arc::new(Mutex::new(axe_collection)))
        .launch();
}
