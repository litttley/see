

extern crate libflate;
use crate::config::Header;
use std::collections::HashMap;
use std::io::{Write};
use libflate::gzip;

#[derive(Default)]
pub struct Response {
    version: String,
    status: i32,
    header: HashMap<String, String>,
    body: Vec<u8>,
    gzip: bool
}

pub enum StatusCode {
    _200,
    _301,
    _400,
    _401,
    _404,
    _405,
    _500
}

const SERVER_NAME: &str = env!("CARGO_PKG_NAME");

impl Response {

    // HTTP response
    pub fn new(status: StatusCode) -> Response {

        let mut response = Response::default();

        response.version = String::from("HTTP/1.1");

        response.status = match status {
            StatusCode::_200 => 200,
            StatusCode::_400 => 400,
            StatusCode::_301 => 301,
            StatusCode::_401 => 401,
            StatusCode::_404 => 404,
            StatusCode::_405 => 405,
            StatusCode::_500 => 500
        };

        // Add service name
        response.header.insert(String::from("Server"), SERVER_NAME.to_string());

        response

    }

    // Set header
    pub fn header(mut self, key: &str, value:  &str) -> Response {

        self.header.insert(key.to_string(), value.to_string());

        self

    }

    // Set multiple header
    pub fn headers(mut self, headers: &Vec<Header>) -> Response {

        for header in headers {
            self.header.insert(header.key.to_string(), header.value.to_string());
        }

        self

    }

    // Set the content-type based on the file extension
    pub fn content_type(mut self, ext: &str) -> Response {

        let value = match &ext.as_ref() {
            &"aac" => "audio/aac",
            &"abw" => "application/x-abiword",
            &"arc" => "application/x-freearc",
            &"avi" => "video/x-msvideo",
            &"azw" => "application/vnd.amazon.ebook",
            &"bin" => "application/octet-stream",
            &"bmp" => "image/bmp",
            &"bz" => "application/x-bzip",
            &"bz2" => "application/x-bzip2",
            &"csh" => "application/x-csh",
            &"css" => "text/css",
            &"csv" => "text/csv",
            &"doc" => "application/msword",
            &"docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            &"eot" => "application/vnd.ms-fontobject",
            &"epub" => "application/epub+zip",
            &"gif" => "image/gif",
            &"htm" => "text/html",
            &"html" => "text/html",
            &"ico" => "image/vnd.microsoft.icon",
            &"ics" => "text/calendar",
            &"jar" => "application/java-archive",
            &"jpeg" => "image/jpeg",
            &"jpg" => "image/jpeg",
            &"js" => "text/javascript",
            &"json" => "application/json",
            &"mjs" => "text/javascript",
            &"mp3" => "audio/mpeg",
            &"mpeg" => "video/mpeg",
            &"mpkg" => "application/vnd.apple.installer+xml",
            &"odp" => "application/vnd.oasis.opendocument.presentation",
            &"ods" => "application/vnd.oasis.opendocument.spreadsheet",
            &"odt" => "application/vnd.oasis.opendocument.text",
            &"oga" => "audio/ogg",
            &"ogv" => "video/ogg",
            &"ogx" => "application/ogg",
            &"otf" => "font/otf",
            &"png" => "image/png",
            &"pdf" => "application/pdf",
            &"ppt" => "application/vnd.ms-powerpoint",
            &"pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            &"rar" => "application/x-rar-compressed",
            &"rtf" => "application/rtf",
            &"sh" => "application/x-sh",
            &"svg" => "image/svg+xml",
            &"swf" => "application/x-shockwave-flash",
            &"tar" => "application/x-tar",
            &"tif" => "image/tiff",
            &"tiff" => "image/tiff",
            &"ttf" => "font/ttf",
            &"txt" => "text/plain",
            &"vsd" => "application/vnd.visio",
            &"wav" => "audio/wav",
            &"weba" => "audio/webm",
            &"webm" => "video/webm",
            &"webp" => "image/webp",
            &"woff" => "font/woff",
            &"woff2" => "font/woff2",
            &"xhtml" => "application/xhtml+xml",
            &"xls" => "application/vnd.ms-excel",
            &"xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            &"xml" => "text/xml",  // application/xml
            &"xul" => "application/vnd.mozilla.xul+xml",
            &"zip" => "application/zip",
            &"3gp" => "video/3gpp",  // audio/video
            &"3g2" => "video/3gpp2",  // audio/3gpp2
            &"7z" => "application/x-7z-compressed",
            _ => "application/octet-stream"
        };

        self.header.insert("Content-Type".to_string(), value.to_string());
        self

    }

    pub fn gzip(mut self, open: bool) -> Response {

        self.gzip = open;
        self

    }

    // Build a complete response
    pub fn body(mut self, data: &[u8]) -> Vec<u8> {

        if self.gzip {
            self.header.insert("Content-Encoding".to_string(), "gzip".to_string());
            self.body = gzip_min(data);
            self.header.insert("Content-Length".to_string(), self.body.len().to_string());
        }else {
            self.body = data[..].iter().cloned().collect();
            self.header.insert("Content-Length".to_string(), data.len().to_string());
        }

        let mut res = String::from("");

        res.push_str(&self.version);
        res.push_str(&format!(" {}\r\n", &self.status.to_string()));
        for (key, value) in self.header.iter() {
            res.push_str(key);
            res.push_str(": ");
            res.push_str(value);
            res.push_str("\r\n");
        }
        res.push_str("\r\n");

        [&res.as_bytes()[..], &self.body[..]].concat()

    }

}


fn gzip_min(data: &[u8]) -> Vec<u8> {
    let mut encoder = gzip::Encoder::new(Vec::new()).unwrap();
    encoder.write_all(data).unwrap();
    encoder.finish().into_result().unwrap()
}

