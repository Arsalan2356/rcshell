use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
pub enum HyprEvent {
    WorkspaceMod,            // workspace changed/creation => only update active
    WindowRemovedFromActive, // window closed/moved => update active + client counts
    WindowAddedToActive,     // window opened => update active + client counts
    ActiveWindow,            // focus changed => update title
    WindowTitle {
        // a window's title changed => update title + that window's tooltip
        address: String,
        title: String,
    },
}

pub fn dispatch_workspace(i: usize) {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap();
    let path = format!("{runtime}/hypr/{his}/.socket.sock");
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ = stream
            .write_all(format!("dispatch hl.dsp.focus({{ workspace = \"{i}\" }})").as_bytes());
    }
}

pub fn dispatch_up() {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap();
    let path = format!("{runtime}/hypr/{his}/.socket.sock");
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ =
            stream.write_all(format!("dispatch hl.dsp.focus({{ workspace = '+1' }})").as_bytes());
    }
}

pub fn dispatch_down() {
    let his = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();
    let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap();
    let path = format!("{runtime}/hypr/{his}/.socket.sock");
    if let Ok(mut stream) = UnixStream::connect(&path) {
        let _ =
            stream.write_all(format!("dispatch hl.dsp.focus({{ workspace = '-1' }})").as_bytes());
    }
}

pub fn spawn_event_listener(sender: UnboundedSender<HyprEvent>) {
    std::thread::spawn(move || {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();

        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap();

        let path = format!("{}/hypr/{}/.socket2.sock", runtime, sig);

        let stream = UnixStream::connect(path).expect("Could not connect to Hyprland Socket");
        let reader = BufReader::new(stream);

        for line in reader.lines() {
            let Ok(line) = line else { break };
            let event = if line.starts_with("workspacev2") || line.starts_with("createworkspacev2")
            {
                HyprEvent::WorkspaceMod
            } else if line.starts_with("closewindow") || line.starts_with("movewindowv2") {
                HyprEvent::WindowRemovedFromActive
            } else if line.starts_with("openwindow") {
                HyprEvent::WindowAddedToActive
            } else if line.starts_with("activewindowv2") {
                HyprEvent::ActiveWindow
            } else if let Some(data) = line.strip_prefix("windowtitlev2>>") {
                // windowtitlev2>>ADDRESS,TITLE
                let (address, title) = data.split_once(',').unwrap_or((data, ""));
                HyprEvent::WindowTitle {
                    address: address.to_string(),
                    title: title.to_string(),
                }
            } else {
                continue;
            };

            if sender.send(event).is_err() {
                break;
            }
        }
    });
}
