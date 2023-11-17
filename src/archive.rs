/// Image archive implementation
use crate::shrx;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ImageArchive {
    path: String,
    max_age: u32,
    fps: u32,
    max_len: u32,
    stop: Arc<Mutex<bool>>,
    thread: Vec<std::thread::JoinHandle<()>>,
}

fn now_ms() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(n) => n.as_millis() as u64,
        Err(_) => 0,
    }
}

fn err(s: &str) -> Box<dyn std::error::Error> {
    Box::<dyn std::error::Error>::from(String::from(s))
}

fn run_thread(
    path: &str,
    max_age: u32,
    fps: u32,
    max_len: u32,
    stop: Arc<Mutex<bool>>,
) -> std::thread::JoinHandle<()> {
    let mut time_points: Vec<u64> = vec![];

    for i in 0..(if fps <= 60 { fps } else { 60 }) {
        time_points.push((i as u64) * 1000 / (fps as u64));
    }

    std::thread::spawn(move || {
        let mut next_frame = now_ms();
        loop {
            {
                let s = stop.lock().unwrap();
                if *s {
                    return ();
                }
            }
        }
    })
}

impl ImageArchive {
    pub fn new(path: &str) -> Result<ImageArchive> {
        shrx::Pattern::new("xxx");
        Ok(ImageArchive {
            path: String::from(path),
            max_age: 86400 * 10,
            fps: 1,
            max_len: 3600,
            stop: Arc::new(Mutex::new(false)),
            thread: vec![],
        })
    }

    pub fn set_fps(&mut self, fps: u32) -> Result<u32> {
        if fps == 0 {
            return Err(err("Invalid frame rate"));
        }

        self.fps = fps;

        Ok(fps)
    }

    pub fn get_fps(&self) -> u32 {
        self.fps
    }

    /// Run image archive in separate thread
    pub fn run(&self) -> Result<()> {
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        {
            let mut s = self.stop.lock().unwrap();

            *s = true;
        }

        if self.thread.len() == 0 {
            return Err(err("Not running"));
        }

        let t = self.thread.pop();
        t.expect("Internal error").join();

        Ok(())
    }

    pub fn add_image(&self, buf: &[u8]) -> Result<String> {
        Ok(String::from("xxx"))
    }
}
