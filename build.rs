use std::env;
use std::error;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

struct FileContent {
    path: String,
    name: String,
    content: Vec<u8>,
    content_type: String,
}

fn load_file(path: &Path, rpath: String) -> Result<FileContent> {
    let mut file = std::fs::File::open(path)?;
    let mut content: Vec<u8> = vec![];
    file.read_to_end(&mut content)?;
    let content_type = mime_type(&rpath);

    Ok(FileContent {
        path: String::from(path.to_str().unwrap()),
        name: rpath,
        content: content,
        content_type: content_type,
    })
}

fn load_dir_recur(res: &mut Vec<FileContent>, path: &Path, rpath: String) -> Result<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let mut rp = rpath.clone();
            let p = entry.path();
            rp += "/";
            rp += entry.path().file_name().unwrap().to_str().unwrap();
            load_dir_recur(res, &p, rp)?;
        }
    } else {
        res.push(load_file(path, rpath)?);
    }

    Ok(())
}

fn load_dir(path: &str) -> Result<Vec<FileContent>> {
    let mut res: Vec<FileContent> = vec![];
    let path = Path::new(path);

    load_dir_recur(&mut res, path, String::from(""))?;

    Ok(res)
}

const MIME_TYPES: [[&str; 2]; 107] = [
    ["html", "text/html"],
    ["htm", "text/html"],
    ["shtml", "text/html"],
    ["css", "text/css"],
    ["xml", "text/xml"],
    ["gif", "image/gif"],
    ["jpeg", "image/jpeg"],
    ["jpg", "image/jpeg"],
    ["js", "application/javascript"],
    ["atom", "application/atom+xml"],
    ["rss", "application/rss+xml"],
    ["mml", "text/mathml"],
    ["txt", "text/plain"],
    ["jad", "text/vnd.sun.j2me.app-descriptor"],
    ["wml", "text/vnd.wap.wml"],
    ["htc", "text/x-component"],
    ["avif", "image/avif"],
    ["png", "image/png"],
    ["svg", "image/svg+xml"],
    ["svgz", "image/svg+xml"],
    ["tif", "image/tiff"],
    ["tiff", "image/tiff"],
    ["wbmp", "image/vnd.wap.wbmp"],
    ["webp", "image/webp"],
    ["ico", "image/x-icon"],
    ["jng", "image/x-jng"],
    ["bmp", "image/x-ms-bmp"],
    ["woff", "font/woff"],
    ["woff2", "font/woff2"],
    ["jar", "application/java-archive"],
    ["war", "application/java-archive"],
    ["ear", "application/java-archive"],
    ["json", "application/json"],
    ["hqx", "application/mac-binhex40"],
    ["doc", "application/msword"],
    ["pdf", "application/pdf"],
    ["ps", "application/postscript"],
    ["eps", "application/postscript"],
    ["ai", "application/postscript"],
    ["rtf", "application/rtf"],
    ["m3u8", "application/vnd.apple.mpegurl"],
    ["kml", "application/vnd.google-earth.kml+xml"],
    ["kmz", "application/vnd.google-earth.kmz"],
    ["xls", "application/vnd.ms-excel"],
    ["eot", "application/vnd.ms-fontobject"],
    ["ppt", "application/vnd.ms-powerpoint"],
    ["odg", "application/vnd.oasis.opendocument.graphics"],
    ["odp", "application/vnd.oasis.opendocument.presentation"],
    ["ods", "application/vnd.oasis.opendocument.spreadsheet"],
    ["odt", "application/vnd.oasis.opendocument.text"],
    ["wmlc", "application/vnd.wap.wmlc"],
    ["wasm", "application/wasm"],
    ["7z", "application/x-7z-compressed"],
    ["cco", "application/x-cocoa"],
    ["jardiff", "application/x-java-archive-diff"],
    ["jnlp", "application/x-java-jnlp-file"],
    ["run", "application/x-makeself"],
    ["pl", "application/x-perl"],
    ["pm", "application/x-perl"],
    ["prc", "application/x-pilot"],
    ["pdb", "application/x-pilot"],
    ["rar", "application/x-rar-compressed"],
    ["rpm", "application/x-redhat-package-manager"],
    ["sea", "application/x-sea"],
    ["swf", "application/x-shockwave-flash"],
    ["sit", "application/x-stuffit"],
    ["tcl", "application/x-tcl"],
    ["tk", "application/x-tcl"],
    ["der", "application/x-x509-ca-cert"],
    ["pem", "application/x-x509-ca-cert"],
    ["crt", "application/x-x509-ca-cert"],
    ["xpi", "application/x-xpinstall"],
    ["xhtml", "application/xhtml+xml"],
    ["xspf", "application/xspf+xml"],
    ["zip", "application/zip"],
    ["bin", "application/octet-stream"],
    ["exe", "application/octet-stream"],
    ["dll", "application/octet-stream"],
    ["deb", "application/octet-stream"],
    ["dmg", "application/octet-stream"],
    ["iso", "application/octet-stream"],
    ["img", "application/octet-stream"],
    ["msi", "application/octet-stream"],
    ["msp", "application/octet-stream"],
    ["msm", "application/octet-stream"],
    ["mid", "audio/midi"],
    ["midi", "audio/midi"],
    ["kar", "audio/midi"],
    ["mp3", "audio/mpeg"],
    ["ogg", "audio/ogg"],
    ["m4a", "audio/x-m4a"],
    ["ra", "audio/x-realaudio"],
    ["3gpp", "video/3gpp"],
    ["3gp", "video/3gpp"],
    ["ts", "video/mp2t"],
    ["mp4", "video/mp4"],
    ["mpeg", "video/mpeg"],
    ["mpg", "video/mpeg"],
    ["mov", "video/quicktime"],
    ["webm", "video/webm"],
    ["flv", "video/x-flv"],
    ["m4v", "video/x-m4v"],
    ["mng", "video/x-mng"],
    ["asx", "video/x-ms-asf"],
    ["asf", "video/x-ms-asf"],
    ["wmv", "video/x-ms-wmv"],
    ["avi", "video/x-msvideo"],
];

fn mime_type(fnm: &str) -> String {
    let name = String::from(fnm);
    let dot_idx = name.rfind(".");

    let ext = match dot_idx {
        Some(pos) => &fnm[pos + 1..fnm.len()],
        None => fnm,
    };

    for entry in MIME_TYPES {
        if ext == entry[0] {
            return String::from(entry[1]);
        }
    }

    String::from("application/octet-string")
}

fn gen_wwwdata(path: &Path, data: Vec<FileContent>) -> Result<()> {
    let mut out = fs::File::create(path)?;
    let mut last_idx = 0;
    let mut info: Vec<(String, String, i32)> = vec![];

    writeln!(
        out,
        "// This file is generated automatically by build.rs, don't edit it"
    )?;
    for fc in &data {
        info.push((
            String::from(&fc.name),
            String::from(&fc.content_type),
            last_idx,
        ));
        writeln!(out, "// File: {}", fc.path)?;
        write!(
            out,
            "const CONTENT_{}: [u8;{}] = [",
            last_idx,
            fc.content.len()
        )?;
        last_idx += 1;

        let mut i = 0;
        for v in &fc.content {
            if i % 16 == 0 {
                write!(out, "\n    ")?;
            }
            i += 1;
            write!(out, "{}, ", v)?;
        }

        writeln!(out, "];")?;
    }

    // Now we can write the function:
    writeln!(out, "pub struct Content {}", "{")?;
    writeln!(out, "    pub content: &'static [u8],")?;
    writeln!(out, "    pub content_type: &'static str,")?;
    writeln!(out, "{}", "}")?;
    writeln!(
        out,
        "pub fn get_file_content(path: &str) -> Option<Content> {}",
        "{"
    )?;
    write!(out, "    return ")?;
    for entry in &info {
        writeln!(out, " if path == \"{}\" {}", entry.0, "{")?;
        writeln!(out, "        Some(Content{}", "{")?;
        writeln!(out, "            content: &CONTENT_{},", entry.2)?;
        writeln!(out, "            content_type: \"{}\",", entry.1)?;
        writeln!(out, "        {})", "}")?;
        write!(out, "    {} else", "}")?;
    }
    writeln!(out, " {} None {};", "{", "}")?;

    writeln!(out, "{}\n", "}")?;

    Ok(())
}

fn main() {
    let data = load_dir("./wwwroot").unwrap();
    for fc in &data {
        println!("cargo:rerun-if-changed={}", fc.path);
    }

    let out_dir = env::var_os("OUT_DIR").unwrap_or(String::from(".").into());
    let dest_path = Path::new(&out_dir).join("wwwdata.rs");
    gen_wwwdata(&dest_path, data).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}
