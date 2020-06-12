use std::{env, fs};
use std::process::{Command, Stdio, ExitStatus};
use std::path::PathBuf;

fn main() {
    let current_dir = env::current_dir().unwrap();
    let web_dir = current_dir.join("web");
    let web_src = web_dir.join("src");
    for entry in fs::read_dir(web_src).unwrap() {
        let path = entry.unwrap().path();
        if !path.ends_with("App.css") {
            println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
        }
    }

    let web_public = web_dir.join("public");
    for entry in fs::read_dir(web_public).unwrap() {
        println!("cargo:rerun-if-changed={}", entry.unwrap().path().to_str().unwrap());
    }

    let out = Command::new("yarn")
        .arg("cargo")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .current_dir(web_dir)
        .output()
        .unwrap();
    if !out.status.success() {
        panic!("Yarn build failure!");
    }
}
