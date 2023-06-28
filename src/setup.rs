use crate::config::Config;
use reqwest::blocking as reqwest;
use std::env;
use std::fs::{create_dir, set_permissions, File, Permissions};
use std::io;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use text_io::read;
use which::which;
const OS: &str = env::consts::OS;

fn go_arch() -> &'static str {
    let arch = env::consts::ARCH;
    match arch {
        "x86_64" => "amd64",
        "x86" => "386",
        "aarch64" => "arm64",
        "arm" => "armv7",
        _ => panic!("Unsupported architecture: {}", arch),
    }
}

fn dl_sing_box() -> Result<String, String> {
    let client = reqwest::Client::new();

    let rsp = client
        .get("https://api.github.com/repos/SagerNet/sing-box/releases/latest")
        // GitHub blocks reqwest user agent.
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/113.0.0.0 Safari/537.36")
        .send()
        .expect("Failed to get latest release.");
    let json: serde_json::Value = rsp.json().expect("Failed to parse JSON.");
    let assets = json["assets"].as_array().unwrap();
    for asset in assets {
        let name = asset["name"].as_str().unwrap();
        if !name.contains(OS) {
            continue;
        }
        if !name.contains(go_arch()) {
            continue;
        }
        // Download file.
        let url = asset["browser_download_url"].as_str().unwrap();
        let mut rsp = client
            .get(url)
            .send()
            .expect("Failed to download sing-box.");
        if !Path::new("bin").exists() {
            create_dir("bin").expect("Failed to create bin directory.");
        }
        let file_name;
        if OS == "windows" {
            file_name = "bin\\sing-box.exe";
        } else {
            file_name = "bin/sing-box";
        }
        if Path::new(file_name).exists() {
            std::fs::remove_file(file_name).expect("Failed to remove old sing-box executable.");
        }
        let mut file = File::create(file_name).expect("Failed to create sing-box executable.");
        io::copy(&mut rsp, &mut file).expect("Failed to write sing-box executable.");
        if OS != "windows" {
            set_permissions(file_name, Permissions::from_mode(0o755)).unwrap();
        }
        return Ok(file_name.to_string());
    }
    Err("Failed to download sing-box.".to_string())
}

fn sing_box() -> String {
    print!("Enter the path to the sing-box executable []: ");
    let mut bin: String = read!("{}\n");
    if bin == "" {
        if which("sing-box").is_err() {
            println!("sing-box not found in PATH, downloading...");
            bin = dl_sing_box().unwrap();
        } else {
            println!("sing-box found in PATH.");
            bin = which("sing-box").unwrap().to_str().unwrap().to_string()
        }
    } else if !Path::new(&bin).exists() {
        println!("sing-box not found in the specified path, downloading...");
        bin = dl_sing_box().unwrap();
    }
    bin
}

pub(crate) fn setup() -> Config {
    let bin = sing_box();
    Config {
        bin,
        server: "".to_string(),
    }
}
