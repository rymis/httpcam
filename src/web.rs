// Web interface related stuff

use std::sync::{Arc, Mutex};
use tinyjson::JsonValue;
mod default_image;
mod static_content;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + 'static>>;

pub struct Server {
    srv: Arc<Impl>,
    workers: Vec<std::thread::JoinHandle<()>>,
    receiver: std::sync::mpsc::Receiver<JsonRequest>,
}

pub struct JsonRequest {
    pub method: String,
    pub args: tinyjson::JsonValue,
    pub result_sender: std::sync::mpsc::Sender<tinyjson::JsonValue>,
}

struct ResponseInfo {
    result: Vec<u8>,
    content_type: String,
    status: i32,
}

pub type APICallback =
    Box<dyn Fn(&tinyjson::JsonValue) -> Result<tinyjson::JsonValue> + Send + Sync + 'static>;

struct Impl {
    srv: Arc<tiny_http::Server>,
    lock: Mutex<bool>,
    last_image: Mutex<Vec<u8>>,
}

fn header(t: &str, v: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(t.as_bytes(), v.as_bytes()).unwrap()
}

impl ResponseInfo {
    fn new(status: i32, content_type: &str, result: Vec<u8>) -> ResponseInfo {
        ResponseInfo {
            status: status,
            content_type: String::from(content_type),
            result: result,
        }
    }

    fn from_string(status: i32, content_type: &str, result: &str) -> ResponseInfo {
        ResponseInfo::new(
            status,
            content_type,
            Vec::<u8>::from(result.to_string().as_bytes()),
        )
    }
}

impl Impl {
    fn worker(&self, sender: Box<std::sync::mpsc::Sender<JsonRequest>>) {
        loop {
            {
                let stop = self.lock.lock().unwrap();
                if *stop {
                    break;
                }
            }

            let raw_req = self.srv.recv_timeout(core::time::Duration::new(1, 0));
            match raw_req {
                Ok(req) => match req {
                    Some(mut req) => match self.process_request(&mut req, &sender) {
                        Ok(content) => {
                            let mut response = tiny_http::Response::from_data(content.result);
                            response.add_header(header("content-type", &content.content_type));
                            match req.respond(response.with_status_code(content.status)) {
                                Ok(_) => (),
                                Err(err) => println!("Error: {}", err),
                            }
                        }
                        Err(err) => println!("Error: {}", err),
                    },
                    None => (),
                },
                Err(err) => println!("Error: {}", err),
            }
        }
    }

    fn process_request(
        &self,
        req: &mut tiny_http::Request,
        sender: &std::sync::mpsc::Sender<JsonRequest>,
    ) -> Result<ResponseInfo> {
        let url = if req.url() == "/" {
            String::from("/index.html")
        } else {
            String::from(req.url())
        };

        println!("{} {}", req.method(), req.url());

        if url.starts_with("/image.jpg") {
            {
                let img = self.last_image.lock().unwrap();
                return Ok(ResponseInfo::new(
                    200,
                    "image/jpeg",
                    Vec::<u8>::from(&**img),
                ));
            }
        } else if url.starts_with("/api/") {
            let method = url[5..url.len()].to_string();

            let req = if *req.method() == tiny_http::Method::Post {
                let mut content = String::new();
                req.as_reader().read_to_string(&mut content)?;
                let r: JsonValue = content.parse()?;
                r
            } else {
                JsonValue::Null {}
            };

            let (snd, rcv) = std::sync::mpsc::channel::<JsonValue>();
            sender.send(JsonRequest {
                method: method,
                result_sender: snd,
                args: req,
            })?;

            let resp = rcv.recv()?;

            return Ok(ResponseInfo::from_string(
                200,
                "application/json",
                &resp.stringify()?,
            ));
        } else {
            let content = static_content::get_file_content(&url);
            match content {
                Some(content) => {
                    return Ok(ResponseInfo::new(
                        200,
                        content.content_type,
                        content.content.to_vec(),
                    ));
                }
                None => (),
            }
        }

        Ok(ResponseInfo::from_string(404, "text/plain", "Not found"))
    }
}

fn start_impl_thread(
    srv: Arc<Impl>,
    sender: &std::sync::mpsc::Sender<JsonRequest>,
) -> std::thread::JoinHandle<()> {
    let s = Box::new(sender.clone());
    std::thread::spawn(move || srv.worker(s))
}

impl Server {
    pub fn new(addr: &str) -> Result<Server> {
        let srv = tiny_http::Server::http(addr);
        let (sender, receiver) = std::sync::mpsc::channel::<JsonRequest>();

        match srv {
            Ok(srv) => {
                let imp = Arc::new(Impl {
                    lock: Mutex::new(false),
                    srv: Arc::new(srv),
                    last_image: Mutex::new(Vec::<u8>::from(default_image::DEFAULT_IMAGE)),
                });
                let mut workers: Vec<std::thread::JoinHandle<()>> = vec![];

                for _ in [0..4] {
                    let r = Arc::clone(&imp);
                    let worker = start_impl_thread(r, &sender);
                    workers.push(worker);
                }

                Ok(Server {
                    srv: imp,
                    workers: workers,
                    receiver: receiver,
                })
            }
            Err(err) => return Err(err),
        }
    }

    pub fn destroy(self) {
        {
            let mut stop = self.srv.lock.lock().unwrap();
            *stop = true;
        }

        for th in self.workers {
            match th.join() {
                Ok(_) => (),
                Err(_err) => println!("Error: can't join thread"),
            }
        }
    }

    pub fn update_image(&self, data: &[u8]) -> Result<()> {
        {
            let mut img = self.srv.last_image.lock().unwrap();
            *img = Vec::<u8>::from(data);
        }

        Ok(())
    }

    pub fn json_request(&self) -> Option<JsonRequest> {
        let res = self
            .receiver
            .recv_timeout(core::time::Duration::from_millis(10));
        match res {
            Ok(res) => Some(res),
            Err(_) => None,
        }
    }
}
