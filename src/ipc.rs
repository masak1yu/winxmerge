use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::Sender;

fn socket_path() -> std::path::PathBuf {
    let user = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
    std::path::PathBuf::from(format!("/tmp/winxmerge-{}.sock", user))
}

/// Try to connect to a running instance and send file paths.
/// Returns Ok(()) if paths were sent successfully (caller should exit).
/// Returns Err if no running instance found.
pub fn try_send(paths: &[String]) -> Result<(), ()> {
    let sock = socket_path();
    match UnixStream::connect(&sock) {
        Ok(mut stream) => {
            let payload = paths.join("\n") + "\n";
            let _ = stream.write_all(payload.as_bytes());
            let _ = stream.flush();
            Ok(())
        }
        Err(_) => Err(()),
    }
}

/// Try to send with retries (waiting for server to start).
pub fn try_send_with_retry(paths: &[String], retries: u32) -> Result<(), ()> {
    for _ in 0..retries {
        if try_send(paths).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(())
}

/// Copy files to a temp directory so git difftool can clean up originals.
/// Returns new paths pointing to the copies.
pub fn copy_to_temp(paths: &[String]) -> Vec<String> {
    let temp_dir = std::env::temp_dir().join("winxmerge-diff");
    let _ = std::fs::create_dir_all(&temp_dir);
    let id = std::process::id();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    paths
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let src = std::path::Path::new(p);
            let name = src
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("file{}", i));
            let dest = temp_dir.join(format!("{}-{}-{}-{}", id, ts, i, name));
            let _ = std::fs::copy(src, &dest);
            dest.to_string_lossy().to_string()
        })
        .collect()
}

/// Spawn a new winxmerge server process in the background.
pub fn spawn_server() {
    let exe = std::env::current_exe().unwrap_or_else(|_| "winxmerge".into());
    let _ = std::process::Command::new(exe)
        .arg("--server")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

/// Start listening for incoming file paths from other instances.
/// Sends received paths through the channel.
pub fn start_listener(tx: Sender<Vec<String>>) {
    let sock = socket_path();
    // Remove stale socket file
    let _ = std::fs::remove_file(&sock);

    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[winxmerge] IPC listen failed: {}", e);
            return;
        }
    };

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                let reader = BufReader::new(stream);
                let paths: Vec<String> = reader
                    .lines()
                    .map_while(Result::ok)
                    .filter(|l| !l.is_empty())
                    .collect();
                if !paths.is_empty() {
                    let _ = tx.send(paths);
                }
            }
        }
    });
}

/// Clean up the socket file (call on app exit).
pub fn cleanup() {
    let _ = std::fs::remove_file(socket_path());
}
