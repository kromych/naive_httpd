use httpd::ThreadPool;
use std::fs;
use std::io::prelude::*;
use std::net::TcpListener;
use std::net::TcpStream;

const MAX_THREADS: usize = 4;
const MAX_REQUESTS: usize = 2;

fn main() {
    env_logger::init();

    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let thread_pool = ThreadPool::new(MAX_THREADS).unwrap();

    for stream in listener.incoming().take(MAX_REQUESTS) {
        let stream = stream.unwrap();

        thread_pool.execute(|| handle_connection(stream));
    }
}

fn handle_connection(mut stream: TcpStream) {
    let mut buf = [0_u8; 1024];

    if stream.read(&mut buf).is_err() {
        return;
    }

    log::debug!("Request: {}", String::from_utf8_lossy(&buf));

    let get = b"GET / HTTP/1.1\r\n";

    let (response_file, mut status_code, mut status_line) = if buf.starts_with(get) {
        ("index.html", 200, "OK")
    } else {
        ("404.html", 404, "Not Found")
    };

    let contents = match fs::read_to_string(response_file) {
        Ok(contents) => contents,
        Err(_) => {
            let contents = "Internal Server Error";
            status_code = 503;
            status_line = "Internal Server Error";

            String::from(contents)
        }
    };

    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Length:{}\r\n\r\n{}",
        status_code,
        status_line,
        contents.len(),
        contents
    );

    if stream.write(response.as_bytes()).is_err() {
        return;
    }

    stream.flush().ok();
}
