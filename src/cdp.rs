use crate::cancelled;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{net::UnixStream, process::CommandExt},
    },
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct Browser {
    child: Child,
    writer: UnixStream,
    reader: BufReader<UnixStream>,
    next_id: u64,
}

fn duplicate(fd: i32) -> Result<OwnedFd, String> {
    // High, CLOEXEC descriptors prevent dup2 collisions with the child's 3/4.
    let copy = unsafe { libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10) };
    if copy < 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(unsafe { OwnedFd::from_raw_fd(copy) })
}

impl Browser {
    pub fn launch(executable: &Path, profile: &Path, headless: bool) -> Result<Self, String> {
        let (writer, child_in) = UnixStream::pair().map_err(|e| e.to_string())?;
        let (reader, child_out) = UnixStream::pair().map_err(|e| e.to_string())?;
        reader
            .set_read_timeout(Some(Duration::from_millis(200)))
            .map_err(|e| e.to_string())?;
        writer
            .set_write_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| e.to_string())?;
        let input = duplicate(child_in.as_raw_fd())?;
        let output = duplicate(child_out.as_raw_fd())?;
        let parent = unsafe { libc::getpid() };
        let mut command = Command::new(executable);
        if headless {
            command.arg("--headless");
        }
        command
            .args([
                "--remote-debugging-pipe",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-extensions",
                "--disable-background-networking",
                "--disable-component-update",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg("about:blank")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        unsafe {
            command.pre_exec(move || {
                if libc::setpgid(0, 0) == -1
                    || libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1
                {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(std::io::Error::other("Parent exited"));
                }
                if libc::dup2(input.as_raw_fd(), 3) == -1 || libc::dup2(output.as_raw_fd(), 4) == -1
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let child = command.spawn().map_err(|_| {
            "Could not start Chromium. Install chromium and run inside your desktop session."
                .to_string()
        })?;
        drop(child_in);
        drop(child_out);
        Ok(Self {
            child,
            writer,
            reader: BufReader::new(reader),
            next_id: 0,
        })
    }

    fn message(&mut self, deadline: Instant) -> Result<Value, String> {
        let mut message = Vec::new();
        loop {
            if cancelled() {
                return Err("Cancelled.".into());
            }
            if Instant::now() >= deadline {
                return Err(
                    "Chromium did not respond within 30 seconds. Close the session and try again."
                        .into(),
                );
            }
            let bytes = match self.reader.fill_buf() {
                Ok([]) => return Err("The article browser closed or could not start.".into()),
                Ok(bytes) => bytes,
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::WouldBlock
                            | std::io::ErrorKind::TimedOut
                            | std::io::ErrorKind::Interrupted
                    ) =>
                {
                    continue
                }
                Err(_) => return Err("Lost the private connection to Chromium.".into()),
            };
            let end = bytes.iter().position(|b| *b == 0);
            let n = end.unwrap_or(bytes.len());
            if message.len() + n > 32 * 1024 * 1024 {
                return Err("Browser response exceeded the 32 MB limit.".into());
            }
            message.extend_from_slice(&bytes[..n]);
            self.reader.consume(n + usize::from(end.is_some()));
            if end.is_some() {
                return serde_json::from_slice(&message)
                    .map_err(|_| "Invalid browser response.".into());
            }
        }
    }

    pub fn call(
        &mut self,
        session: Option<&str>,
        method: &str,
        params: Value,
    ) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        let mut request = json!({"id":id,"method":method,"params":params});
        if let Some(s) = session {
            request["sessionId"] = json!(s);
        }
        let mut bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
        bytes.push(0);
        self.writer
            .write_all(&bytes)
            .map_err(|_| "Could not send a command to Chromium.".to_string())?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            let response = self.message(deadline)?;
            if response["id"].as_u64() != Some(id) {
                continue;
            }
            if response.get("error").is_some() {
                return Err(format!(
                    "Chromium could not complete {method}. Try reopening the article."
                ));
            }
            return Ok(response["result"].clone());
        }
    }

    pub fn page(&mut self, url: &str) -> Result<String, String> {
        let target = self.call(None, "Target.createTarget", json!({"url":url}))?;
        let target_id = target["targetId"]
            .as_str()
            .ok_or("Missing browser target.")?;
        let attached = self.call(
            None,
            "Target.attachToTarget",
            json!({"targetId":target_id,"flatten":true}),
        )?;
        attached["sessionId"]
            .as_str()
            .map(String::from)
            .ok_or("Missing browser session.".into())
    }

    pub fn pdf(&mut self, html: &str, paper: &str) -> Result<Vec<u8>, String> {
        use base64::Engine;
        let session = self.page("about:blank")?;
        let s = Some(session.as_str());
        self.call(s, "Page.enable", json!({}))?;
        self.call(s, "Network.enable", json!({}))?;
        self.call(s, "Network.setBlockedURLs", json!({"urls":["*"]}))?;
        self.call(
            s,
            "Emulation.setScriptExecutionDisabled",
            json!({"value":true}),
        )?;
        let tree = self.call(s, "Page.getFrameTree", json!({}))?;
        let frame = tree["frameTree"]["frame"]["id"]
            .as_str()
            .ok_or("Missing print frame.")?;
        self.call(
            s,
            "Page.setDocumentContent",
            json!({"frameId":frame,"html":html}),
        )?;
        let result = self.call(s, "Page.printToPDF", json!({"printBackground":true,"displayHeaderFooter":false,"preferCSSPageSize":true,"paperWidth":if paper=="A4" {8.2677} else {8.5},"paperHeight":if paper=="A4" {11.6929} else {11.0}}))?;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(result["data"].as_str().ok_or("No PDF returned.")?)
            .map_err(|_| "Invalid PDF data.")?;
        if !bytes.starts_with(b"%PDF-") {
            return Err("Chromium returned an invalid PDF.".into());
        }
        self.call(s, "Page.close", json!({}))?;
        Ok(bytes)
    }
}

impl Drop for Browser {
    fn drop(&mut self) {
        // Only our dedicated process group. Never signal the user's browser.
        let group = -(self.child.id() as i32);
        unsafe {
            libc::kill(group, libc::SIGTERM);
        }
        // Keep the leader unreaped until the group is killed, preventing PID
        // reuse and covering descendants that ignore TERM after leader exit.
        std::thread::sleep(Duration::from_secs(1));
        unsafe {
            libc::kill(group, libc::SIGKILL);
        }
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_pipe_framing_and_print_contract() {
        let profile = tempfile::tempdir().unwrap();
        let exe = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake-browser.cjs");
        let mut browser = Browser::launch(&exe, profile.path(), true).unwrap();
        assert_eq!(
            browser.page("https://x.com/example/status/123").unwrap(),
            "session-1"
        );
        assert_eq!(
            browser
                .pdf("<html><body>fixture</body></html>", "Letter")
                .unwrap(),
            b"%PDF-mock"
        );
        assert!(browser.call(None, "Reject.me", json!({})).is_err());
        let pid = browser.child.id();
        drop(browser);
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
        let calls = std::fs::read_to_string(profile.path().join("calls.jsonl")).unwrap();
        assert!(calls.contains("Network.setBlockedURLs"));
        assert!(calls.contains("Emulation.setScriptExecutionDisabled"));
        assert!(calls.contains("Page.printToPDF"));
    }
    #[test]
    fn closed_pipe_is_an_error() {
        let profile = tempfile::tempdir().unwrap();
        let mut browser = Browser::launch(Path::new("/bin/true"), profile.path(), false).unwrap();
        assert!(browser.page("about:blank").is_err());
    }
}
