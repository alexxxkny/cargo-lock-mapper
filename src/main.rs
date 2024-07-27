use clap::Parser;
use std::{collections::HashMap, fs};
use toml_edit::{
    DocumentMut, Item, Value,
};

#[derive(Parser)]
struct Args {
    #[arg(short = 'l')]
    lock: String,
    #[arg(short = 't')]
    toml: String,
    #[arg(short = 'o')]
    output: String
}

fn get_locked_versions(path: &str) -> HashMap<String, String> {
    let lock = fs::read_to_string(path)
        .expect("Failed to read source .lock file.")
        .parse::<DocumentMut>()
        .unwrap();
    let packages = lock.as_table()["package"].as_array_of_tables().unwrap();

    let mut locked_versions = HashMap::new();
    for package in packages {
        locked_versions.insert(
            package["name"].as_str().unwrap().to_owned(),
            package["version"].as_str().unwrap().to_owned(),
        );
    }

    locked_versions
}

fn main() {
    let args = Args::parse();
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let locked_versions = get_locked_versions(&args.lock);

    let mut cargo = fs::read_to_string(&args.toml)
        .unwrap()
        .parse::<DocumentMut>()
        .unwrap();
    let dependencies = cargo
        .get_mut("workspace")
        .unwrap()
        .get_mut("dependencies")
        .unwrap();
    for (key, item) in dependencies.as_table_mut().unwrap().iter_mut() {
        if item.is_inline_table() {
            let table = item.as_inline_table_mut().unwrap();
            let name = table
                .get("package")
                .map_or(key.get(), |package| package.as_str().unwrap());
            let locked_version = if let Some(v) = locked_versions.get(name) {
                v
            } else {
                log::warn!("{}: SKIP - not defined in .lock", name);
                continue;
            };

            if table.get("path").is_some() || table.get("git").is_some() {
                log::warn!("{}: SKIP - path or git key are defined", name);
                continue;
            }

            log::info!("{}: -> {}", name, locked_version);
            table.insert("version", locked_version.into());
        } else {
            let locked_version = if let Some(v) = locked_versions.get(key.get()) {
                v
            } else {
                log::warn!("{}: SKIP - not defined in .lock", key.get());
                continue;
            };

            log::info!("{}: -> {}", key, locked_version);
            *item = Item::Value(Value::from(locked_version));
        }
    }

    fs::write(&args.output, cargo.to_string().as_bytes())
        .expect("Failed to write output file.");
}
