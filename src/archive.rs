/// Image archive implementation
use crate::shrx;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::io::Write;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct ImageArchive {
    imp: Arc<Mutex<Impl>>,

    thread: Vec<std::thread::JoinHandle<()>>,
}

struct Impl {
    path: String,
    max_age: u32,
    fps: u32,
    max_len: u32,
    stop: bool,
    img: Vec<u8>,
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

impl Impl {
    fn get_fps(&self) -> u32 {
        if self.fps == 0 {
            1
        } else if self.fps > 60 {
            60
        } else {
            self.fps
        }
    }

    fn next_frame_impl(&mut self, time_point: u64) -> Result<()> {
        // We should save the frame:
        let mut filename = std::path::PathBuf::new();
        filename.push(&self.path);
        filename.push(format!("frame_{}.jpg", time_point));

        let mut f = std::fs::File::create(filename.as_path())?;
        f.write(&self.img)?;

        Ok(())
    }

    fn next_frame(&mut self, time_point: u64) {
        match self.next_frame_impl(time_point) {
            Ok(()) => (),
            Err(err) => println!("Can't save frame: {}", err),
        }
    }
}

fn run_thread(arch: Arc<Mutex<Impl>>) -> std::thread::JoinHandle<()> {
    let mut time_points: Vec<u64> = vec![];

    {
        let a = arch.lock().unwrap();

        for i in 0..a.get_fps() {
            time_points.push((i as u64) * 1000 / (a.get_fps() as u64));
        }
    }

    std::thread::spawn(move || {
        let now = now_ms();
        let n_ms = now % 1000;
        let mut tpidx = 0;
        while tpidx < time_points.len() && time_points[tpidx] < n_ms {
            tpidx += 1;
        }

        let mut next_frame = if tpidx == time_points.len() {
            now - n_ms + time_points[0] + 1000
        } else {
            now - n_ms + time_points[tpidx]
        };
        tpidx = (tpidx + 1) % time_points.len();

        loop {
            {
                let a = arch.lock().unwrap();
                if a.stop {
                    return ();
                }
            }

            let now = now_ms();
            if now >= next_frame {
                {
                    let mut a = arch.lock().unwrap();
                    a.next_frame(next_frame);
                }

                next_frame = now - now % 1000 + time_points[tpidx];
                tpidx = (tpidx + 1) % time_points.len();
            }

            std::thread::sleep(std::time::Duration::from_micros(2));
        }
    })
}

impl ImageArchive {
    pub fn new(path: &str) -> Result<ImageArchive> {
        shrx::Pattern::new("xxx");
        let imp = Arc::new(Mutex::new(Impl{
            path: String::from(path),
            max_age: 86400 * 10,
            fps: 1,
            max_len: 3600,
            stop: false,
            img: vec![],
        }));

        Ok(ImageArchive {
            thread: vec![],
            imp: imp.clone(),
        })
    }

    pub fn set_fps(&mut self, fps: u32) -> Result<u32> {
        if fps == 0 {
            return Err(err("Invalid frame rate"));
        }

        {
            let mut i = self.imp.lock().unwrap();

            i.fps = fps;
        }

        Ok(fps)
    }

    pub fn get_fps(&self) -> u32 {
        let i = self.imp.lock().unwrap();
        i.fps
    }

    /// Run image archive in separate thread
    pub fn run(&mut self) -> Result<()> {
        self.thread.push(run_thread(self.imp.clone()));
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        {
            let mut i = self.imp.lock().unwrap();

            i.stop = true;
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
