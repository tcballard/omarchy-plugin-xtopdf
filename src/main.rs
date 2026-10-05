mod article;
mod cdp;
use article::Article;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, Read, Write},
    os::{
        fd::AsRawFd,
        unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

static CANCELLED: AtomicBool = AtomicBool::new(false);
pub fn cancelled() -> bool {
    CANCELLED.load(Ordering::Relaxed)
}
extern "C" fn cancel_signal(_: i32) {
    CANCELLED.store(true, Ordering::Relaxed);
}

fn emit(value: Value) {
    let mut out = std::io::stdout().lock();
    if writeln!(out, "{value}").and_then(|_| out.flush()).is_err() {
        CANCELLED.store(true, Ordering::Relaxed);
    }
}

fn private_dir(path: &Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("Private data directory must be absolute.".into());
    }
    if !path.exists() {
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
            .map_err(|_| "Could not create private browser directory.")?;
    }
    let m = fs::symlink_metadata(path).map_err(|_| "Could not inspect private directory.")?;
    if !m.is_dir() || m.file_type().is_symlink() || m.uid() != unsafe { libc::geteuid() } {
        return Err("Browser directory must be a real directory owned by you.".into());
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| "Could not protect private browser directory.".into())
}

fn profile() -> Result<(PathBuf, File), String> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/share")
        });
    let root = base.join("x-to-pdf");
    private_dir(&root)?;
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root.join("session.lock"))
        .map_err(|_| "Could not open session lock.")?;
    let metadata = lock
        .metadata()
        .map_err(|_| "Could not inspect session lock.")?;
    if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err("Invalid session lock.".into());
    }
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err("Another X to PDF session is open. Close it before starting a new one.".into());
    }
    let directory = root.join("chromium");
    private_dir(&directory)?;
    Ok((directory, lock))
}

fn browser_executable() -> Result<PathBuf, String> {
    [
        "/usr/lib/chromium/chromium",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ]
    .iter()
    .map(PathBuf::from)
    .find(|p| p.is_file())
    .ok_or("Chromium is missing. Install the chromium package, then try again.".into())
}

fn read_capped(reader: &mut impl BufRead, limit: usize) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::new();
    loop {
        let bytes = reader.fill_buf().map_err(|_| "Could not read command.")?;
        if bytes.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Err("Incomplete command.".into())
            };
        }
        let end = bytes.iter().position(|b| *b == b'\n');
        let n = end.unwrap_or(bytes.len());
        if line.len() + n > limit {
            return Err("Command exceeded size limit.".into());
        }
        line.extend_from_slice(&bytes[..n]);
        reader.consume(n + usize::from(end.is_some()));
        if end.is_some() {
            return Ok(Some(line));
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
enum Request {
    Capture,
    Export { path: String, paper: String },
    Cancel,
}

fn save_pdf(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if !path.is_absolute() || path.extension().and_then(|e| e.to_str()) != Some("pdf") {
        return Err("Choose an absolute filename ending in .pdf.".into());
    }
    let parent = path.parent().ok_or("Invalid output path.")?;
    if !parent.is_dir() {
        return Err("The destination folder does not exist.".into());
    }
    if bytes.len() > 24 * 1024 * 1024 || !bytes.starts_with(b"%PDF-") {
        return Err("PDF is invalid or too large.".into());
    }
    let mut file = tempfile::NamedTempFile::new_in(parent)
        .map_err(|_| "Cannot write to the destination folder.")?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|_| "Could not finish writing the PDF.")?;
    file.persist_noclobber(path).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            "That file already exists. Choose a new filename.".to_string()
        } else {
            "Could not save the PDF.".to_string()
        }
    })?;
    Ok(())
}

fn capture(browser: &mut cdp::Browser, session: &str) -> Result<Article, String> {
    // Validate the current location before running the extractor, even after navigation.
    let location = browser.call(
        Some(session),
        "Runtime.evaluate",
        json!({"expression":"location.href","returnByValue":true,"timeout":5000}),
    )?;
    let source = article::article_url(
        location["result"]["value"]
            .as_str()
            .ok_or("The article page is not ready.")?,
    )?;
    let tree = browser.call(Some(session), "Page.getFrameTree", json!({}))?;
    let frame = tree["frameTree"]["frame"]["id"]
        .as_str()
        .ok_or("The article frame is not ready.")?;
    let world = browser.call(
        Some(session),
        "Page.createIsolatedWorld",
        json!({"frameId":frame,"worldName":"x-to-pdf-extractor"}),
    )?;
    let result = browser.call(Some(session), "Runtime.evaluate", json!({"expression":include_str!("../assets/extract.js"),"contextId":world["executionContextId"],"returnByValue":true,"timeout":5000}))?;
    let data = &result["result"]["value"];
    if let Some(error) = data["error"].as_str() {
        return Err(error.chars().take(400).collect());
    }
    let encoded = serde_json::to_vec(&data["article"]).map_err(|_| "Invalid article data.")?;
    if encoded.len() > 2 * 1024 * 1024 {
        return Err("Article exceeds the 2 MB capture limit.".into());
    }
    let mut article: Article =
        serde_json::from_slice(&encoded).map_err(|_| "The article format is unsupported.")?;
    article.source = source;
    article.html("Letter")?;
    Ok(article)
}

fn session(url: &str, demo: bool) -> Result<(), String> {
    let url = if demo {
        "about:blank".into()
    } else {
        article::article_url(url)?
    };
    let executable = browser_executable()?;
    let (directory, _lock) = profile()?;
    let mut browser = cdp::Browser::launch(&executable, &directory, false)?;
    let page = browser.page(&url)?;
    browser.call(Some(&page), "Page.enable", json!({}))?;
    emit(
        json!({"event":"ready","message":if demo {"Fictional demo is ready. Click Capture."} else {"In the article browser, sign in if needed and let the full article load. Then click Capture here."}}),
    );
    let (sender, receiver) = mpsc::sync_channel(4);
    std::thread::spawn(move || {
        let mut stdin = std::io::stdin().lock();
        loop {
            match read_capped(&mut stdin, 16384) {
                Ok(Some(line)) => {
                    if sender
                        .send(
                            serde_json::from_slice::<Request>(&line)
                                .map_err(|_| "Invalid command.".to_string()),
                        )
                        .is_err()
                    {
                        break;
                    }
                }
                _ => {
                    CANCELLED.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
    });
    let deadline = Instant::now() + Duration::from_secs(15 * 60);
    let mut article: Option<Article> = None;
    while !cancelled() && Instant::now() < deadline {
        let request = match receiver.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                emit(json!({"event":"error","message":e}));
                continue;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(_) => break,
        };
        let result: Result<(), String> = match request {
            Request::Cancel => break,
            Request::Capture => {
                article = None;
                let result = if demo {
                    serde_json::from_str(include_str!("../demo/fixtures/article.json"))
                        .map_err(|e| e.to_string())
                } else {
                    capture(&mut browser, &page)
                };
                result.map(|a: Article| {
                    emit(json!({"event":"captured","title":a.title,"byline":a.byline,"preview":a.preview(),"blocks":a.content.len(),"message":"Review the text before saving. Capture again if the article was still loading."}));
                    article = Some(a);
                })
            }
            Request::Export { path, paper } => (|| {
                let a = article
                    .as_ref()
                    .ok_or("Capture and review an article first.")?;
                let html = a.html(&paper)?;
                // Printing uses a fresh headless profile: no X login state and
                // no dependence on headful Chromium's PDF support.
                let print_profile = tempfile::tempdir()
                    .map_err(|_| "Could not create the private print session.")?;
                let mut printer = cdp::Browser::launch(&executable, print_profile.path(), true)?;
                let bytes = printer.pdf(&html, &paper)?;
                if cancelled() {
                    return Err("Cancelled.".into());
                }
                save_pdf(Path::new(&path), &bytes)?;
                emit(
                    json!({"event":"saved","path":path,"message":"PDF saved. Open it to check the layout."}),
                );
                Ok(())
            })(),
        };
        if let Err(message) = result {
            emit(json!({"event":"error","message" : message}));
        }
    }
    if Instant::now() >= deadline {
        emit(
            json!({"event":"error","message":"Session timed out after 15 minutes. Reopen the article to continue."}),
        );
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--session") if args.len() == 3 => session(&args[2], false),
        Some("--demo") if args.len() == 2 => session("", true),
        Some("--render-fixture") if args.len() == 3 => {
            let file = File::open(&args[2]).map_err(|e| e.to_string())?;
            let mut text = String::new();
            file.take(2 * 1024 * 1024 + 1)
                .read_to_string(&mut text)
                .map_err(|e| e.to_string())?;
            if text.len() > 2 * 1024 * 1024 {
                return Err("Fixture too large.".into());
            }
            let a: Article = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            println!("{}", a.html("Letter")?);
            Ok(())
        }
        Some("--check") => {
            let path = browser_executable()?;
            emit(json!({"event":"ready","browser":path}));
            Ok(())
        }
        _ => {
            println!("x-to-pdf --session URL | --demo | --check | --render-fixture FILE\nSession stdin: JSON lines with action capture, export (path, paper), or cancel.\nChromium sandbox remains enabled; run as your desktop user.");
            Ok(())
        }
    }
}

fn main() {
    unsafe {
        libc::signal(libc::SIGTERM, cancel_signal as *const () as usize);
        libc::signal(libc::SIGINT, cancel_signal as *const () as usize);
    }
    if let Err(message) = run() {
        emit(json!({"event":"error","message":message}));
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_overwrite_or_symlink_following() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("article.pdf");
        save_pdf(&path, b"%PDF-test").unwrap();
        assert!(save_pdf(&path, b"%PDF-new").is_err());
        assert_eq!(fs::read(&path).unwrap(), b"%PDF-test");
        let link = d.path().join("link.pdf");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(save_pdf(&link, b"%PDF-new").is_err());
        assert!(save_pdf(&d.path().join("bad.pdf"), b"not pdf").is_err());
        assert!(save_pdf(Path::new("relative.pdf"), b"%PDF-test").is_err());
    }
    #[test]
    fn input_is_bounded_and_complete() {
        assert_eq!(
            read_capped(&mut &b"ok\nrest"[..], 2).unwrap(),
            Some(b"ok".to_vec())
        );
        assert!(read_capped(&mut &b"longgg\n"[..], 2).is_err());
        assert!(read_capped(&mut &b"unfinished"[..], 100).is_err());
        assert!(serde_json::from_str::<Request>(
            "{\"action\":\"export\",\"path\":\"/tmp/x.pdf\",\"paper\":\"A4\",\"command\":\"evil\"}"
        )
        .is_err());
    }
    #[test]
    fn profile_rejects_symlink() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("link");
        std::os::unix::fs::symlink(d.path(), &p).unwrap();
        assert!(private_dir(&p).is_err());
    }
}
