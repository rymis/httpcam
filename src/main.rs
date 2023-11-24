use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
use nokhwa::{nokhwa_initialize, Camera};
use std::error::Error;
use tinyjson::JsonValue;

use argh::FromArgs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub mod archive;
pub mod mjpeg;
pub mod shrx;
pub mod web;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn save_file(name: &str, data: &[u8]) -> Result<()> {
    let path = Path::new(name);
    let mut f = File::create(path)?;

    f.write_all(data)?;

    Ok(())
}

#[derive(FromArgs)]
/// Simple HTTP webcam interface
struct CmdLine {
    /// listen address in the form addr:port
    #[argh(option, short = 'a', default = "String::from(\"0.0.0.0:8080\")")]
    address: String,

    /// index of a camera to use
    #[argh(option, short = 'c', default = "0")]
    camera: u32,

    /// get <fps> frames per second
    #[argh(option, default = "4")]
    fps: u32,

    /// write images archive into directory
    #[argh(option, short = 'o')]
    output: Option<String>,

    /// maximum image age in archive in hours
    #[argh(option, default = "24")]
    max_age: u32,

    /// select resolution
    #[argh(option)]
    resolution: Option<String>,

    /// list all devices and known resolutions
    #[argh(switch, short = 'l')]
    list: bool,
}

fn camera_format_string(fmt: &nokhwa::utils::CameraFormat) -> String {
    let mut res = String::new();

    res += &fmt.width().to_string();
    res += "x";
    res += &fmt.height().to_string();
    res += "/";
    res += &fmt.frame_rate().to_string();

    res
}

fn list_cameras(cameras: &Vec<nokhwa::utils::CameraInfo>) -> Result<()> {
    for info in cameras {
        println!("{}", info);
        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
        let mut camera = Camera::new(info.index().clone(), requested);
        match camera {
            Ok(mut camera) => {
                let formats = camera.compatible_camera_formats();
                match formats {
                    Ok(formats) => {
                        for fmt in &formats {
                            println!("    {} ({})", camera_format_string(&fmt), fmt.format());
                        }
                    }
                    Err(_) => (),
                }
            }
            Err(_) => (),
        }
    }

    Ok(())
}

fn api_error(msg: &str) -> JsonValue {
    let mut res = std::collections::HashMap::<String, JsonValue>::new();
    res.insert(String::from("error"), JsonValue::String(msg.to_string()));
    JsonValue::Object(res)
}

fn api_ping(req: &JsonValue) -> Result<JsonValue> {
    Ok(req.clone())
}

fn api_list_resolutions(cam: &mut Camera, _req: &JsonValue) -> Result<JsonValue> {
    let mut res: Vec<JsonValue> = vec![];
    let formats = cam.compatible_camera_formats()?;
    for fmt in &formats {
        let mut f = std::collections::HashMap::<String, JsonValue>::new();
        f.insert(String::from("width"), JsonValue::Number(fmt.width() as f64));
        f.insert(
            String::from("height"),
            JsonValue::Number(fmt.height() as f64),
        );
        f.insert(
            String::from("fps"),
            JsonValue::Number(fmt.frame_rate() as f64),
        );
        f.insert(
            String::from("format"),
            JsonValue::String(fmt.format().to_string()),
        );

        res.push(JsonValue::Object(f));
    }

    Ok(JsonValue::Array(res))
}

fn flag_to_string(f: &[nokhwa::utils::KnownCameraControlFlag]) -> String {
    for flag in f {
        return String::from(match flag {
            nokhwa::utils::KnownCameraControlFlag::Automatic => "automatic",
            nokhwa::utils::KnownCameraControlFlag::Manual => "manual",
            nokhwa::utils::KnownCameraControlFlag::Continuous => "continuous",
            nokhwa::utils::KnownCameraControlFlag::ReadOnly => "readonly",
            nokhwa::utils::KnownCameraControlFlag::WriteOnly => "writeonly",
            nokhwa::utils::KnownCameraControlFlag::Volatile => "volatile",
            nokhwa::utils::KnownCameraControlFlag::Disabled => "disabled",
        });
    }
    String::from("unknown")
}

fn api_list_controls(cam: &mut Camera, _req: &JsonValue) -> Result<JsonValue> {
    let mut res: Vec<JsonValue> = vec![];
    for control in cam.camera_controls()? {
        let mut ctrl = std::collections::HashMap::<String, JsonValue>::new();
        let descr = control.description();

        ctrl.insert(
            String::from("name"),
            JsonValue::String(control.name().to_string()),
        );
        ctrl.insert(
            String::from("flag"),
            JsonValue::String(flag_to_string(control.flag())),
        );

        // We can set only known types of values
        let add = match descr {
            nokhwa::utils::ControlValueDescription::IntegerRange {
                min, max, value, ..
            } => {
                ctrl.insert(
                    String::from("type"),
                    JsonValue::String(String::from("number")),
                );
                ctrl.insert(String::from("min"), JsonValue::Number(*min as f64));
                ctrl.insert(String::from("max"), JsonValue::Number(*max as f64));
                ctrl.insert(String::from("value"), JsonValue::Number(*value as f64));
                true
            }
            nokhwa::utils::ControlValueDescription::FloatRange {
                min, max, value, ..
            } => {
                ctrl.insert(
                    String::from("type"),
                    JsonValue::String(String::from("number")),
                );
                ctrl.insert(String::from("min"), JsonValue::Number(*min));
                ctrl.insert(String::from("max"), JsonValue::Number(*max));
                ctrl.insert(String::from("value"), JsonValue::Number(*value));
                true
            }

            _ => false,
        };

        if add {
            res.push(JsonValue::Object(ctrl));
        }
    }

    Ok(JsonValue::Array(res))
}

fn api_set_control(cam: &mut Camera, req: &JsonValue) -> Result<JsonValue> {
    Ok(JsonValue::Boolean(true))
}

fn api<F>(mut cb: F, req: &JsonValue) -> JsonValue
where
    F: FnMut(&JsonValue) -> Result<JsonValue>,
{
    match cb(req) {
        Ok(res) => res,
        Err(err) => api_error(&err.to_string()),
    }
}

fn main_err() -> Result<()> {
    let args: CmdLine = argh::from_env();

    nokhwa_initialize(|r: bool| {
        println!("Result: {}", r);
    });

    let cameras = nokhwa::query(nokhwa::utils::ApiBackend::Auto)?;

    if args.list {
        list_cameras(&cameras)?;
        return Ok(());
    }

    for x in cameras {
        println!("{}", x);
    }

    let srv = web::Server::new("127.0.0.1:8080")?;

    let requested =
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution);
    let index = CameraIndex::Index(args.camera);
    let mut camera = Camera::new(index, requested)?;
    let mut archive: Option<archive::ImageArchive> = match args.output {
        Some(path) => Some(archive::ImageArchive::new(&path)?),
        None => None,
    };

    match archive {
        Some(ref mut arch) => {
            // TODO: set archive parameters
            arch.run();
        },
        None => (),
    };

    match args.resolution {
        Some(r) => {
            let formats = camera.compatible_camera_formats()?;
            let mut found = false;
            for fmt in &formats {
                if camera_format_string(fmt) == r {
                    camera.set_resolution(fmt.resolution())?;
                    camera.set_frame_rate(fmt.frame_rate())?;
                    camera.set_frame_format(fmt.format())?;
                    found = true;
                    break;
                }
            }

            if !found {
                return Err(<Box<dyn Error>>::from(String::from(
                    "Camera format is not found!",
                )));
            }
        }
        None => (),
    };

    camera.open_stream()?;

    println!(
        "Resolution: {}/{}",
        camera.resolution(),
        camera.frame_rate()
    );

    loop {
        let req = srv.json_request();
        match req {
            Some(req) => {
                let res = if req.method == "ping" {
                    api(api_ping, &req.args)
                } else if req.method == "list_controls" {
                    api(
                        |req: &JsonValue| -> Result<JsonValue> {
                            api_list_controls(&mut camera, req)
                        },
                        &req.args,
                    )
                } else if req.method == "list_resolution" {
                    api(
                        |req: &JsonValue| -> Result<JsonValue> {
                            api_list_resolutions(&mut camera, req)
                        },
                        &req.args,
                    )
                } else if req.method == "set_control" {
                    api(
                        |req: &JsonValue| -> Result<JsonValue> {
                            api_set_control(&mut camera, req)
                        },
                        &req.args,
                    )
                } else {
                    api_error("Unknown method")
                };
                req.result_sender.send(res)?;
            }
            None => (),
        }

        let frame = camera.frame()?;
        println!(
            "Frame: {} {}",
            frame.resolution(),
            frame.source_frame_format()
        );
        srv.update_image(frame.buffer())?;

        match archive {
            Some(ref mut a) => {
                println!("Written image {}", a.add_image(frame.buffer())?);
            }
            None => (),
        };
    }

    // save_file("test.jpg", frame.buffer());
}

fn main() {
    main_err().unwrap();
}
