

extern crate chrono;
use std::u8;
use std::sync::Arc;
use std::{fs, fs::File};
use std::env;
use std::{process, process::Command};
use std::path::Path;
use std::io::prelude::*;
use std::fmt::Write as FmtWrite;
use std::net::{TcpStream, TcpListener};
use std::thread::JoinHandle;
use chrono::{DateTime, Local};
mod response;
use response::{StatusCode, Response};
mod request;
use request::Request;
mod html;
use html::TEMPLATE;
mod config;
use config::{ServerConfig, DirectoryOption, RewriteType};
mod log;
mod app;
use app::App;

#[cfg(target_os = "macos")]
static PID_PATH: &str = "/usr/local/var/run/see.pid";
#[cfg(target_os = "linux")]
static PID_PATH: &str = "/var/run/see.pid";
#[cfg(target_os = "windows")]
static PID_PATH: &str = "./see.pid";

const DEFAULT_CONFIG_PATH: &str = "config.yml";
const DEFAULT_PORT: i64 = 80;

fn main() {

    let app = App::new();

    if app.help() {
        return app.print_help();
    }

    if app.version() {
        return app.print_version();
    }

    if app.stop() {
        return stop_daemon();
    }

    let mut configs: Vec<Arc<Vec<ServerConfig>>>;
    let current_buff = env::current_dir()
        .unwrap();
    let current_dir = current_buff.to_str()
        .unwrap();

    if app.start() {

        let mut config = ServerConfig::default();
        config.root = String::from(current_dir);
        config.directory = Some(DirectoryOption {
            time: true,
            size: true
        });
        config.methods = vec![
            String::from("GET"),
            String::from("HEAD"),
        ];
        config.listen = match app.port() {
            Ok(result) => {
                match result {
                    Some(port) => port,
                    None => DEFAULT_PORT
                }
            },
            Err(arg) => {
                eprintln!("unable to bind to port \"{}\"", arg);
                process::exit(1);
            }
        };
        configs = vec![Arc::new(vec![config])];

    }else {

        let mut config_path = match app.config() {
            Some(path) => path,
            None => String::from(DEFAULT_CONFIG_PATH)
        };

        config_path = fill_path(current_dir, &config_path);
        configs = match ServerConfig::new(&config_path) {
            Ok(config) => config,
            Err(msg) => {
                eprintln!("{}", msg);
                process::exit(1);
            }
        };

        // Check configuration file
        if app.test() {
            return println!("the configuration file {} syntax is ok", config_path);
        }

    }

    if app.detach() {
        return start_daemon(&app.args, app.detach_args());
    }

    let mut tasks: Vec<JoinHandle<()>> = vec![];
    let start = app.start();

    for config in configs {

        let task = std::thread::spawn(move || {

            let listen = config[0].listen;
            let address = format!("0.0.0.0:{}", listen);

            match TcpListener::bind(&address) {
                Ok(listener) => {
                    if start {
                        println!("Serving path   : \x1b[92m{}\x1b[0m",  &config[0].root);
                        if listen != 80 {
                            println!("Serving address: \x1b[93mhttp://127.0.0.1:{}\x1b[0m",  listen);
                        }else {
                            println!("Serving address: \x1b[93mhttp://127.0.0.1\x1b[0m");
                        }
                    }
                    incoming(listener, config);
                },
                Err(err) => {
                    eprintln!("Binding {} failed", address);
                    eprintln!("{:?}", err);
                    process::exit(1);
                }
            };

        });

        tasks.push(task);

    }

    for task in tasks {
        task.join().unwrap();
    }

}


fn incoming(listener: TcpListener, configs: Arc<Vec<ServerConfig>>) {
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let configs = configs.clone();
            std::thread::spawn(|| {
                handle_connection(stream, configs);
            });
        }
    }
}


fn start_daemon(args: &Vec<String>, detach: [&str; 2]) {

    let args = args
        .iter()
        .filter(|item| {
            return *item != detach[0] && *item != detach[1]
        })
        .cloned()
        .collect::<Vec<String>>();

    let child = Command::new(&args[0])
        .args(&args[1..])
        .spawn();

    match child {
        Ok(child) => {
            let mut pid = File::create(PID_PATH)
                .unwrap();
            write!(pid, "{}", child.id()).unwrap();
        },
        Err(e) => {
            eprintln!("{}", e);
        }
    }

}


fn stop_daemon() {
    match fs::read_to_string(PID_PATH) {
        Ok(pid) => {
            let kill = Command::new("kill")
                .arg(pid)
                .status();
            match kill {
                Ok(status) => {
                    if status.success() {
                        if let Err(e) = fs::remove_file(PID_PATH) {
                            eprintln!("{}", e);
                        }
                    }
                },
                Err(e) => {
                    eprintln!("{}", e);
                }
            }
        },
        Err(e) => {
            eprintln!("open \"{}\" failed, {:?}", PID_PATH, e.to_string());
        }
    }
}


fn handle_connection(mut stream: TcpStream, configs: Arc<Vec<ServerConfig>>) {

    let mut buffer = [0; 512];
    stream.read(&mut buffer).unwrap();

    if u8::min_value() == buffer[0] {
        stream.flush().unwrap();
        return;
    }

    let mut res: Vec<u8> = vec![];
    let req = Request::new(&buffer[..]);

    if let Some(host) = req.headers.get("host") {
        let host = &host.replace(&format!(":{}", configs[0].listen), "");
        'configs: for config in configs.iter() {
            if let Some(hosts) = &config.hosts {
                for val in hosts {
                    if val == host {
                        res = output(&req, &config, &stream);
                        break 'configs;
                    }
                }
            }
        }
    }

    if res.len() == 0 {
        for conf in configs.iter() {
            if let None = conf.hosts {
                res = output(&req, &conf, &stream);
                break;
            }
        }
    }

    if res.len() != 0 {
        stream.write(&res).unwrap();
    }
    stream.flush().unwrap();

}


fn output(request: &Request, config: &ServerConfig, stream: &TcpStream) -> Vec<u8> {

    // Not allowed method
    let allow = config.methods.iter().find(|item| {
        return **item == request.method;
    });
    if let None = allow {
        if let Some(log) = &config.log.error {
            log.write(&request.method, 405, &request.path);
        }
        return Response::new(StatusCode::_405, &config.headers)
            .text("405");
    }

    // A Host header field must be sent in all HTTP/1.1 request messages
    if let None = request.headers.get("host") {
        if let Some(log) = &config.log.error {
            log.write(&request.method, 400, &request.path);
        }
        return Response::new(StatusCode::_400, &config.headers)
            .text("400");
    }

    if let Some(auth) = &config.auth {
        let authorization = request.headers.get("authorization");
        if let Some(value) = authorization {
            if auth != value {
                if let Some(log) = &config.log.error {
                    log.write(&request.method, 401, &request.path);
                }
                return Response::new(StatusCode::_401, &config.headers)
                    .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
                    .text("401");
            }
        }else {
            if let Some(log) = &config.log.error {
                log.write(&request.method, 401, &request.path);
            }
            return Response::new(StatusCode::_401, &config.headers)
                .header("WWW-Authenticate", "Basic realm=\"User Visible Realm\"")
                .text("401");
        }
    }

    if let Some(rewrite) = &config.rewrite {
        if let Some(rewrite) = rewrite.get(&request.path) {
            match rewrite.status {
                RewriteType::_301 => {
                    if let Some(log) = &config.log.success {
                        log.write(&request.method, 301, &request.path);
                    }
                    return Response::new(StatusCode::_301, &config.headers)
                        .rewrite(rewrite.url.to_string());
                }
                RewriteType::_302 => {
                    if let Some(log) = &config.log.success {
                        log.write(&request.method, 302, &request.path);
                    }
                    return Response::new(StatusCode::_302, &config.headers)
                        .rewrite(rewrite.url.to_string());
                }
                RewriteType::Path => {
//                    request.path = rewrite.url.to_string();
                }
            }
        }
    }

    let cur_path = String::from(".") + &request.path;
    let path_buff = Path::new(&config.root)
        .join(&cur_path);
    let path = path_buff
        .to_str()
        .unwrap();

    match fs::metadata(&path) {
        Ok(meta) => {
            if meta.is_dir() {
                if request.path.chars().last().unwrap_or('.') == '/' {
                    if let Some(index) = &config.index {
                        let index_path = fill_path(&path, &index);
                        match File::open(index_path) {
                            Ok(file) => {
                                if let Some(log) = &config.log.success {
                                    log.write(&request.method, 200, &request.path);
                                }
                                let ext = get_extension(index);
                                Response::new(StatusCode::_200, &config.headers)
                                    .content_type(ext)
                                    .gzip(can_use_gzip(&request, &config, &ext))
                                    .file(&stream, file);
                                return vec![]
                            },
                            Err(_) => {
                                if let Some(log) = &config.log.error {
                                    log.write(&request.method, 404, &request.path);
                                }
                                return output_404(&config);
                            }
                        }
                    }
                    if let Some(option) = &config.directory {
                        if let Some(log) = &config.log.success {
                            log.write(&request.method, 200, &request.path);
                        }
                        return Response::new(StatusCode::_200, &config.headers)
                            .content_type("html")
                            .body(response_dir_html(&path, &request.path, option.time, option.size).as_bytes())
                    }
                    if let Some(log) = &config.log.error {
                        log.write(&request.method, 404, &request.path);
                    }
                    return output_404(&config);
                }else {
                    if let Some(log) = &config.log.success {
                        log.write(&request.method, 301, &request.path);
                    }
                    let aims;
                    if let Some(query) = &request.query {
                        aims = format!("{}/?{}", request.path, query);
                    }else {
                        aims = format!("{}/", request.path);
                    }
                    return Response::new(StatusCode::_301, &config.headers)
                        .rewrite(aims);
                }
            }else {
                match File::open(&path) {
                    Ok(file) => {
                        if let Some(log) = &config.log.success {
                            log.write(&request.method, 200, &request.path);
                        }
                        let ext = get_extension(&path);
                        Response::new(StatusCode::_200, &config.headers)
                            .content_type(&ext)
                            .gzip(can_use_gzip(&request, &config, &ext))
                            .file(&stream, file);
                        return vec![]
                    },
                    Err(_) => {
                        if let Some(log) = &config.log.error {
                            log.write(&request.method, 500, &request.path);
                        }
                        return output_500(&config);
                    }
                }
            }
        },
        Err(_) => {
            if let Some(exts) = &config.extensions {
                match fallbacks(&path, exts) {
                    Ok(fallback) => {
                        if let Some(log) = &config.log.success {
                            log.write(&request.method, 200, &request.path);
                        }
                        Response::new(StatusCode::_200, &config.headers)
                            .content_type(&fallback.ext)
                            .gzip(can_use_gzip(&request, &config, &fallback.ext))
                            .file(&stream, fallback.file);
                        return vec![]
                    },
                    Err(_) => {
                        if let Some(log) = &config.log.error {
                            log.write(&request.method, 404, &request.path);
                        }
                        return output_404(&config);
                    }
                }
            }else {
                if let Some(log) = &config.log.error {
                    log.write(&request.method, 404, &request.path);
                }
                return output_404(&config);
            }
        }
    };

}


fn can_use_gzip(request: &Request, config: &ServerConfig, ext: &str) -> bool {

    if let Some(exts) = &config.gzip {
        let allow = exts.iter().find(|item| {
            return *item == ext
        });
        if let None = allow {
            return false
        }
    }else {
        return false
    }

    let encoding = if let Some(val) = request.headers.get("accept-encoding") {
        val
    }else {
        return false
    };

    let ways: Vec<&str> = encoding.split(", ").collect();
    for way in ways {
        if way == "gzip" {
            return true
        }
    }

    return false

}


pub fn fill_path(root: &str, file: &str) -> String {

    if Path::new(&file).is_absolute() {
        file.to_string()
    } else {
        let buff = Path::new(&root)
            .join(&file);
        let path = buff
            .to_str()
            .unwrap();
        path.to_string()
    }

}


fn output_404(config: &ServerConfig) -> Vec<u8> {

    let res = Response::new(StatusCode::_404, &config.headers);

    if let Some(file) = &config.error._404 {
        match fs::read(&file) {
            Ok(data) => {
                return res
                    .content_type(get_extension(file))
                    .body(&data[..]);
            },
            Err(_) => {
                return res.text("404");
            }
        }
    }else {
        return res.text("404");
    }

}


fn output_500(config: &ServerConfig) -> Vec<u8> {

    let res = Response::new(StatusCode::_500, &config.headers);

    if let Some(file) = &config.error._500 {
        match fs::read(&file) {
            Ok(data) => {
                return res
                    .content_type(get_extension(file))
                    .body(&data[..])
            },
            Err(_) => {
                return res.text("500");
            }
        }
    }else {
        return res.text("500")
    }

}

// Get the file extension
fn get_extension(path: &str) -> &str {

    let extension = Path::new(path)
        .extension();
    
    if let Some(ext) = extension {
        match ext.to_str() {
            Some(e) => e,
            None => ""
        }
    } else {
        ""
    }

}


struct Fallbacks {
    file: File,
    ext: String
}

fn fallbacks(file: &str, exts: &Vec<String>) -> Result<Fallbacks, ()> {

    let has_ext = Path::new(&file)
        .extension();
    if let Some(_) = has_ext {
        return Err(());
    }

    for x in exts {
        let path = format!("{}.{}", file, x);
        if let Ok(file) = File::open(&path) {
            return Ok(Fallbacks {
                file,
                ext: x.to_string()
            });
        }
    }

    return Err(());

}


fn response_dir_html(path: &str, title: &str, show_time: bool, show_size: bool) -> String {

    let dir = match fs::read_dir(path) {
        Ok(dir) => dir,
        Err(_) => {
            return String::new()
        }
    };

    let (
        mut files,
        mut main,
        mut first
    ) = (
        String::new(),
        "auto auto 1fr",
        "1 / 4"
    );

    if !show_time && !show_size{
        main = "auto";
        first = "1 / 2";
    }else if (!show_time && show_size) || (show_time && !show_size) {
        main = "auto 1fr";
        first = "1 / 3";
    }

    for x in dir {

        let entry = match x {
            Ok(entry) => entry,
            Err(_) => continue
        }.path();

        let filename = match entry.file_name() {
            Some(d) => {
                match d.to_str() {
                    Some(n) => {
                        if entry.is_dir() {
                            format!("{}/", n)
                        }else {
                            n.to_string()
                        }
                    },
                    None => continue
                }
            },
            None => continue
        };

        let _ = write!(files, "<a href=\"{}\">{}</a>", filename, filename);

        if show_size || show_time {
            if let Ok(meta) = fs::metadata(&entry) {
                if show_time {
                    let time = if let Ok(time) = meta.modified() {
                        let datetime: DateTime<Local> = DateTime::from(time);
                        datetime.format("%Y-%m-%d %H:%M").to_string()
                    }else {
                        String::new()
                    };
                    files.push_str(&format!("<time>{}</time>", time));
                }
                if show_size {
                    let size = if entry.is_file() {
                        bytes_to_size(meta.len() as f64)
                    }else {
                        String::new()
                    };
                    files.push_str(&format!("<span>{}</span>", size));
                }
            }
        }

    }

    TEMPLATE
        .replace("{title}", title)
        .replace("{main}", main)
        .replace("{first}", first)
        .replace("{files}", &files)

}


fn bytes_to_size(bytes: f64) -> String {
    let k = 1024_f64;
    let sizes = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
    if bytes <= 1_f64 {
        return format!("{:.2} B", bytes)
    }
    let i = (bytes.ln() / k.ln()) as i32;
    format!("{:.2} {}", bytes / k.powi(i), sizes[i as usize])
}


#[test]
fn test_get_extension() {
    assert_eq!(get_extension("index.html"), "html");
    assert_eq!(get_extension("/index/index.rs"), "rs");
    assert_eq!(get_extension(""), "");
    assert_eq!(get_extension("index"), "");
}

#[test]
fn test_bytes_to_size() {
    assert_eq!(bytes_to_size(0_f64), "0.00 B");
    assert_eq!(bytes_to_size(0.5_f64), "0.50 B");
    assert_eq!(bytes_to_size(1_f64), "1.00 B");
    assert_eq!(bytes_to_size(12_f64), "12.00 B");
    assert_eq!(bytes_to_size(1024_f64), "1.00 KB");
    assert_eq!(bytes_to_size(1025_f64), "1.00 KB");
    assert_eq!(bytes_to_size(123456_f64), "120.56 KB");
    assert_eq!(bytes_to_size(99999999_f64), "95.37 MB");
    assert_eq!(bytes_to_size(99999999999_f64), "93.13 GB");
}
