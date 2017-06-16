// Setup Rocket
#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(custom_derive)]

extern crate abes_axe;
extern crate clap;
extern crate rocket;
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use std::collections::{HashMap};
use std::sync::{Mutex, Arc};
use abes_axe::{Axe};
use rocket::{State};
use rocket_contrib::{JSON, Value};
use abes_axe::options::Options;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash,Hasher};

type AxeMap = HashMap<String,Axe>;
type AxeMapRef = Arc<Mutex<AxeMap>>;

#[derive(Deserialize)]
struct RepoPath {
    repo: String
}

#[get("/")]
fn index(axeset: State<AxeMapRef>) -> JSON<Value> {
    let axes = axeset.lock().unwrap();
    JSON(json!({"count": axes.len()}))
}

#[get("/repo/<id>")]
fn get_repository(axeset: State<AxeMapRef>, id: &str) -> JSON<Value> {
    let mut axes = axeset.lock().unwrap();
    if let Some(_) = axes.get_mut(&String::from(id)) {
        JSON(json!({"status": "good"}))
    }else{
        JSON(json!({"status": "bad"}))        
    }
}

#[get("/repo/<id>/csv")]
fn get_csv(axeset: State<AxeMapRef>, id: &str) -> String {
    let mut axes = axeset.lock().unwrap();
    if let Some(axe) = axes.get_mut(&String::from(id)) {
        axe.csv().unwrap()
    }else{
        String::from("BADNESS")
    }
}

#[post("/repo/new", format="application/json", data="<path>")]
fn add_repository(path: JSON<RepoPath>, axeset: State<AxeMapRef>) -> JSON<Value> {
    let mut axes = axeset.lock().unwrap();
    let repo_path = path.into_inner().repo;
    let options = Options { repo_path: repo_path.clone(), ..Default::default() };
    let axe = Axe::new(Some(options)).unwrap();
    let hash = repo_hash(&repo_path).to_string();
    axes.insert(hash.clone(), axe);
    // how to make the stuff happen?
    JSON(json!(
        {"repo" : repo_path,
         "id" : hash
        }))
}

fn repo_hash(path: &String) -> u64 {
    let mut s = DefaultHasher::new();
    path.hash(&mut s);
    s.finish()
}

fn main() {
    let axe_collection: HashMap<String, Axe> = HashMap::new();
    
    // Run server here with axe in place....
    rocket::ignite()
        .mount("/", routes![index])
        .mount("/", routes![add_repository])
        .mount("/", routes![get_repository])
        .mount("/", routes![get_csv])
        .manage(Arc::new(Mutex::new(axe_collection)))
        .launch();
}
