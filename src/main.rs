#![feature(stmt_expr_attributes)]
pub mod config;
mod setup;

use std::fs::File;
use std::io::{self, Seek, Write};
use std::path::Path;
use std::process::Command;
use std::sync::mpsc::channel;
use std::{thread, time};

use config::Config;
use ctrlc;
use text_io::read;
#[cfg(unix)]
use users::get_current_uid;

const UPDATE_URL: &str =
    "https://raw.githubusercontent.com/teppyboy/everything-v2ray/master/client/profile/sfa/";
const SERVER_LIST: [(&str, &str); 2] = [("us", "vpn-us.json"), ("vn", "vpn-vn.json")];

fn server_from_name(name: &str) -> String {
    SERVER_LIST
        .iter()
        .find(|&i| i.0.trim() == name.trim())
        .expect("Invalid server name, valid names are us and vn.")
        .1
        .to_string()
}

fn select_server(config: &Config) -> String {
    print!(
        "Select which server you want to connect to [{}]: ",
        config.server
    );
    let mut server: String = read!("{}\n");
    match server.as_str() {
        "" => {
            if config.server.as_str() == "" {
                println!("Selecting default server (US)...");
                server = "us".to_string();
            } else {
                server = config.server.clone();
            }
        }
        _ => (),
    }
    server
}

fn update_server_config(server_file: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}{}", UPDATE_URL, server_file);
    let mut rsp = client
        .get(&url)
        .send()
        .expect("Failed to download server configuration.");
    if !Path::new("servers").exists() {
        std::fs::create_dir("servers").expect("Failed to create servers directory.");
    }
    let mut file = File::create(format!("servers/{}", server_file))
        .expect("Failed to create server configuration.");
    io::copy(&mut rsp, &mut file).expect("Failed to write server configuration.");
    Ok(())
}

fn main() {
    println!("Tahoe VPN client for everything-v2ray.");
    println!();
    // Check if the config file exists.
    let config_file = Path::new("config.json");
    let mut config: Config;
    let mut file;
    if config_file.exists() {
        // Read the config file.
        println!("Config file exists.");
        file = File::options()
            .read(true)
            .write(true)
            .open(config_file)
            .unwrap();
        config = serde_json::from_reader(&file).unwrap();
    } else {
        // Create the config file.
        println!("Config file does not exist, entering first time setup.");
        file = File::create(config_file).unwrap();
        config = setup::setup();
    }
    let server = select_server(&config);
    let server_file = server_from_name(&server);
    config.server = server;
    file.rewind().unwrap();
    file.write_all(serde_json::to_string_pretty(&config).unwrap().as_bytes())
        .unwrap();
    // Close the file.
    drop(file);
    let mut update_cfg = false;
    if !Path::new(&format!("servers/{}", server_file)).exists() {
        println!("Updating server configuration...");
        update_server_config(&server_file).unwrap();
        update_cfg = true;
    }
    println!("Starting sing-box...");
    let mut cmd;
    #[cfg(unix)]
    if get_current_uid() != 0 {
        cmd = Command::new("sudo");
        cmd.arg(&config.bin);
    } else {
        cmd = Command::new(&config.bin);
    }
    #[cfg(windows)]
    cmd = Command::new(&config.bin);
    cmd.arg("run")
        .arg("-c")
        .arg(format!("servers/{}", server_file));
    let mut proc = cmd.spawn().expect("Failed to start sing-box.");
    let (tx, rx) = channel();
    ctrlc::set_handler(move || tx.send(()).expect("Could not send signal on channel."))
        .expect("Error setting Ctrl-C handler");
    if !update_cfg {
        println!("Waiting for 5 seconds before updating server configuration...");
        thread::sleep(time::Duration::from_secs(5));
        println!("Updating server configuration...");
        update_server_config(&server_file).unwrap();
    }
    rx.recv().expect("Could not receive from channel.");
    println!("Stopping sing-box...");
    proc.kill().expect("Failed to kill sing-box.");
    println!("Done.");
}
