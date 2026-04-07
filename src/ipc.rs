use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::Sender;

fn socket_path() -> std::path::PathBuf {
    let user = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
    std::path::PathBuf::from(format!("/tmp/winxmerge-{}.sock", user))
}

/// Try to connect to a running instance and send file paths.
/// Returns Ok(()) if paths were sent successfully (caller should exit).
/// Returns Err if no running instance found (caller should become the primary).
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

/// Start listening for incoming file paths from other instances.
/// Sends received (left, right) pairs through the channel.
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
