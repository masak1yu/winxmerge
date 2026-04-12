use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::mpsc::Sender;

fn socket_path() -> std::path::PathBuf {
    let user = std::env::var("USER").unwrap_or_else(|_| "default".to_string());
    std::path::PathBuf::from(format!("/tmp/winxmerge-{}.sock", user))
}

/// Try to connect to a running instance and send file path pairs.
/// Each pair is (original_path, temp_path), sent as "ORIGINAL\tTEMP\n".
/// Returns Ok(()) if paths were sent successfully (caller should exit).
/// Returns Err if no running instance found.
pub fn try_send(pairs: &[(String, String)]) -> Result<(), ()> {
    let sock = socket_path();
    match UnixStream::connect(&sock) {
        Ok(mut stream) => {
            let payload: String = pairs
                .iter()
                .map(|(orig, temp)| format!("{}\t{}", orig, temp))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            let _ = stream.write_all(payload.as_bytes());
            let _ = stream.flush();
            Ok(())
        }
        Err(_) => Err(()),
    }
}

/// Copy files to a temp directory so git difftool can clean up originals.
/// Returns pairs of (original_path, temp_path).
pub fn copy_to_temp(paths: &[String]) -> Vec<(String, String)> {
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
            (p.clone(), dest.to_string_lossy().to_string())
        })
        .collect()
}

/// Start listening for incoming file path pairs from other instances.
/// Each line is "ORIGINAL\tTEMP". Sends received pairs through the channel.
pub fn start_listener(tx: Sender<Vec<(String, String)>>) {
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
                let pairs: Vec<(String, String)> = reader
                    .lines()
                    .map_while(Result::ok)
                    .filter(|l| !l.is_empty())
                    .map(|line| {
                        if let Some((orig, temp)) = line.split_once('\t') {
                            (orig.to_string(), temp.to_string())
                        } else {
                            (line.clone(), line)
                        }
                    })
                    .collect();
                if !pairs.is_empty() {
                    let _ = tx.send(pairs);
                }
            }
        }
    });
}

/// Clean up the socket file (call on app exit).
pub fn cleanup() {
    let _ = std::fs::remove_file(socket_path());
}

/// Path to the Finder Sync pending-compare request file in the shared App Group container.
fn finder_request_path() -> Option<std::path::PathBuf> {
    let home = std::env::var("HOME").ok()?;
    Some(
        std::path::PathBuf::from(home).join(
            "Library/Group Containers/group.io.github.masak1yu.winxmerge/pending-compare.txt",
        ),
    )
}

/// Check for a pending compare request from the Finder Sync extension.
/// Returns file paths if a request is found, and removes the request file.
pub fn check_finder_request() -> Option<Vec<String>> {
    let path = finder_request_path()?;
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let paths: Vec<String> = content
        .lines()
        .map(|l| l.to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if paths.is_empty() { None } else { Some(paths) }
}
