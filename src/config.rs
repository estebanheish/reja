use serde::Deserialize;
use std::{collections::HashMap, env, fs::read_to_string};

#[derive(Deserialize, Debug)]
pub struct Profile {
    pub model: String,
    pub system: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Config {
    profile: HashMap<String, Profile>,
}

#[allow(deprecated)]
pub fn profile() -> Profile {
    let s = read_to_string(env::home_dir().unwrap().join(".config/reja/reja.toml")).unwrap();
    let mut config: Config = toml::from_str(&s).unwrap();
    if let Some(sel) = env::args().nth(1) {
        config.profile.remove(&sel).expect("profile not found")
    } else {
        config
            .profile
            .remove("default")
            .expect("there is no default profile in the config")
    }
}
