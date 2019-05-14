

extern crate libflate;
use crate::config::Header;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use libflate::gzip;
use std::net::TcpStream;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;

#[derive(Default, Debug)]
pub struct Response {
    version: &'static str,
    status: i32,
    header: HashMap<String, String>,
    body: Vec<u8>,
    gzip: bool
}

pub enum StatusCode {
    _200,
    _301,
    _302,
    _400,
    _401,
    _404,
    _405,
    _500
}

const SERVER_NAME: &str = env!("CARGO_PKG_NAME");

impl Response {

    // HTTP response
    pub fn new(status: StatusCode, headers: &Vec<Header>) -> Response {

        let mut response = Response::default();

        response.version = "HTTP/1.1";

        response.status = match status {
            StatusCode::_200 => 200,
            StatusCode::_400 => 400,
            StatusCode::_301 => 301,
            StatusCode::_302 => 302,
            StatusCode::_401 => 401,
            StatusCode::_404 => 404,
            StatusCode::_405 => 405,
            StatusCode::_500 => 500
        };

        // Add service name
        response.header.insert(String::from("Server"), SERVER_NAME.to_string());

        for header in headers.iter() {
            response.header.insert(
                header.key.to_string(),
                header.value.to_string()
            );
        }

        response

    }

    // Set header
    pub fn header(mut self, key: &str, value:  &str) -> Response {

        self.header.insert(key.to_string(), value.to_string());

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

    pub fn rewrite(mut self, location: String) -> Vec<u8> {

        self.header.insert("location".to_string(), location);

        let mut res = String::new();

        let _ = write!(res, "{} {}\r\n", self.version, self.status);

        for (key, value) in self.header.iter() {
            let _ = write!(res, "{}: {}\r\n", key, value);
        }

        res.push_str("\r\n");

        res.as_bytes().to_vec()

    }

    pub fn text(mut self, text: &str) -> Vec<u8> {

        self.body = text.as_bytes().to_vec();
        self.header.insert("Content-Type".to_string(), "text/plain".to_string());
        self.header.insert("Content-Length".to_string(), self.body.len().to_string());

        let mut res = String::new();

        let _ = write!(res, "{} {}\r\n", self.version, self.status);

        for (key, value) in self.header.iter() {
            let _ = write!(res, "{}: {}\r\n", key, value);
        }

        res.push_str("\r\n");

        [&res.as_bytes()[..], &self.body[..]].concat()

    }

    pub fn body(mut self, data: &[u8]) -> Vec<u8> {

        if self.gzip {
            if let Ok(d) = gzip_min(data) {
                self.header.insert("Content-Encoding".to_string(), "gzip".to_string());
                self.body = d;
            }else {
                self.body = data.to_vec();
            }
        }else {
            self.body = data.to_vec();
        }
        self.header.insert("Content-Length".to_string(), self.body.len().to_string());

        let mut res = String::new();

        let _ = write!(res, "{} {}\r\n", self.version, self.status);

        for (key, value) in self.header.iter() {
            let _ = write!(res, "{}: {}\r\n", key, value);
        }

        res.push_str("\r\n");

        [&res.as_bytes()[..], &self.body[..]].concat()

    }

    pub fn file(mut self, mut stream: &TcpStream, file: File) {

        let meta = file.metadata().unwrap();
        if self.gzip {
            self.header.insert("Content-Encoding".to_string(), "gzip".to_string());
        }
//        self.header.insert("Transfer-Encoding".to_string(), "chunked".to_string());
        self.header.insert("Content-Length".to_string(), format!("{}", meta.len()));

        let mut res = String::new();
        let _ = write!(res, "{} {}\r\n", self.version, self.status);
        for (key, value) in self.header.iter() {
            let _ = write!(res, "{}: {}\r\n", key, value);
        }
        res.push_str("\r\n");
        stream.write(res.as_bytes()).unwrap();

        loop {
            let mut render = BufReader::new(&file);
            if let Ok(data) = render.fill_buf() {
                if data.len() != 0 {
                    stream.write(data).unwrap();
                }else {
                    break;
                }
            }else {
                break;
            }
        }

    }

}


fn gzip_min(data: &[u8]) -> Result<Vec<u8>, ()> {
    let mut encoder = match gzip::Encoder::new(Vec::new()) {
        Ok(encoder) => encoder,
        Err(_) => {
            return Err(());
        }
    };
    if let Err(_) = encoder.write_all(data) {
        return Err(());
    }
    if let Ok(min) = encoder.finish().into_result() {
        Ok(min)
    }else {
        Err(())
    }
}


//#[cfg(test)]
//mod tests {
//
//    use crate::response::Response;
//    use crate::response::StatusCode;
//    use crate::response::gzip_min;
//
//    #[test]
//    fn test_build_response() {
//        let res = Response::new(StatusCode::_200)
//            .header("hello", "world")
//            .body(b"200");
//    }
//
//}

