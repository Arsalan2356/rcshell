use std::io::{BufRead, BufReader};
use std::os::unix::net::UnixStream;
use tokio::sync::mpsc::Sender;

#[derive(Debug)]
pub enum HyprEvent {
    WorkspaceMod,            // workspace changed/creation => only update active
    WindowRemovedFromActive, // window closed/moved => update active + client counts
    WindowAddedToActive,     // window opened => update active + client counts
    ActiveWindow,            // workspace not changed, but focus changed => update title
}

pub fn spawn_event_listener(sender: Sender<HyprEvent>) {
    std::thread::spawn(move || {
        let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").unwrap();

        let runtime = std::env::var("XDG_RUNTIME_DIR").unwrap();

        let path = format!("{}/hypr/{}/.socket2.sock", runtime, sig);

        let stream = UnixStream::connect(path).expect("Could not connect to Hyprland socket2");
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
            } else {
                continue;
            };

            if sender.blocking_send(event).is_err() {
                break;
            }
        }
    });
}
