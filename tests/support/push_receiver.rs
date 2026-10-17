use std::{
    collections::HashMap,
    io::{Read, Write},
    net::TcpListener,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use libflate::gzip::Decoder;
use prost::Message;
use pyroscope::encode::gen::push::PushRequest;

pub struct CapturedPush {
    pub path: String,
    pub headers: HashMap<String, String>,
    pub request: PushRequest,
}

pub struct PushReceiver {
    pub url: String,
    worker: JoinHandle<CapturedPush>,
}

impl PushReceiver {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind local upload receiver");
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let worker = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "no profile upload received");
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept upload: {error}"),
                }
            };
            // Accepted sockets can inherit the listener's nonblocking mode.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut bytes = Vec::new();
            let (header_len, path, headers, content_len) = loop {
                let mut chunk = [0; 4096];
                let read = stream.read(&mut chunk).expect("read upload headers");
                assert!(read > 0, "upload connection closed before headers");
                bytes.extend_from_slice(&chunk[..read]);
                assert!(
                    bytes.len() <= 64 * 1024,
                    "upload headers exceed fixture limit"
                );
                let mut header_slots = [httparse::EMPTY_HEADER; 64];
                let mut request = httparse::Request::new(&mut header_slots);
                if let httparse::Status::Complete(header_len) =
                    request.parse(&bytes).expect("parse HTTP request")
                {
                    let path = request.path.unwrap().to_owned();
                    let headers: HashMap<_, _> = request
                        .headers
                        .iter()
                        .map(|header| {
                            (
                                header.name.to_ascii_lowercase(),
                                std::str::from_utf8(header.value).unwrap().to_owned(),
                            )
                        })
                        .collect();
                    let content_len: usize = headers
                        .get("content-length")
                        .expect("known-size body")
                        .parse()
                        .unwrap();
                    assert!(
                        content_len <= 16 * 1024 * 1024,
                        "upload body exceeds fixture limit"
                    );
                    break (header_len, path, headers, content_len);
                }
            };
            while bytes.len() < header_len + content_len {
                let mut chunk = [0; 4096];
                let read = stream.read(&mut chunk).expect("read upload body");
                assert!(read > 0, "upload connection closed before body");
                bytes.extend_from_slice(&chunk[..read]);
            }
            let mut decoded = Vec::new();
            Decoder::new(&bytes[header_len..header_len + content_len])
                .unwrap()
                .read_to_end(&mut decoded)
                .unwrap();
            let request = PushRequest::decode(decoded.as_slice()).expect("decode Pyroscope push");
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .unwrap();
            CapturedPush {
                path,
                headers,
                request,
            }
        });
        Self { url, worker }
    }

    pub fn finish(self) -> CapturedPush {
        self.worker.join().expect("upload receiver completed")
    }
}
