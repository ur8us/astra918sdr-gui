use astra918_host::{Client, Device, Receiver};
use std::{
    sync::mpsc::{self, Receiver as Queue, SyncSender},
    thread,
    time::{Duration, Instant},
};
pub enum Request {
    Discover,
    Connect { simulator: bool, target: String },
    Disconnect,
    Command(u8, Vec<u8>),
}
#[derive(Clone, Default)]
pub struct Snapshot {
    pub devices: Vec<Device>,
    pub discovered: bool,
    pub radio: Option<Receiver>,
    pub message: String,
    pub connected: bool,
}
pub struct Worker {
    pub requests: SyncSender<Request>,
    pub snapshots: Queue<Snapshot>,
}
impl Worker {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::sync_channel(16);
        let (updates, snapshots) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut client: Option<Client> = None;
            let mut state = Snapshot::default();
            let mut poll = Instant::now();
            loop {
                match rx.recv_timeout(Duration::from_millis(20)) {
                    Ok(Request::Discover) => {
                        // The bridge owns vendor interface 4 while SDR Console runs.
                        // Prefer its AST1 proxy over a second, conflicting USB claim.
                        let bridge =
                            Client::tcp("127.0.0.1:30433").and_then(|mut client| client.state());
                        if bridge.is_ok() {
                            state.devices = vec![Device {
                                serial: "bridge".into(),
                                label: "Astra918 via SDR Console bridge".into(),
                            }];
                        } else {
                            match astra918_host::devices() {
                                Ok(d) => state.devices = d,
                                Err(e) => state.message = e.to_string(),
                            }
                        }
                        state.discovered = true;
                    }
                    Ok(Request::Disconnect) => {
                        client = None;
                        state.radio = None;
                        state.connected = false;
                        state.message = "Disconnected".into();
                    }
                    Ok(Request::Connect { simulator, target }) => {
                        client = None;
                        state.radio = None;
                        state.connected = false;
                        let result = if target == "bridge" {
                            Client::tcp("127.0.0.1:30433")
                        } else if simulator && cfg!(debug_assertions) {
                            Client::tcp(&target)
                        } else {
                            Client::usb(&target)
                        };
                        match result.and_then(|mut c| {
                            let s = c.state()?;
                            Ok((c, s))
                        }) {
                            Ok((c, s)) => {
                                client = Some(c);
                                state.radio = Some(s);
                                state.connected = true;
                                state.message = "Connected; receiver settings adopted".into();
                            }
                            Err(e) => state.message = format!("Connection failed: {e:#}"),
                        }
                    }
                    Ok(Request::Command(cmd, payload)) => {
                        if let Some(c) = client.as_mut() {
                            match c.command(cmd, &payload) {
                                Ok(_) => {
                                    state.message = if cmd == 0x36 {
                                        "Settings saved to receiver".into()
                                    } else {
                                        "Applied".into()
                                    }
                                }
                                Err(e) => state.message = format!("Not applied: {e:#}"),
                            }
                            poll = Instant::now() - Duration::from_secs(1);
                        }
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
                if poll.elapsed() >= Duration::from_millis(200) {
                    poll = Instant::now();
                    if let Some(c) = client.as_mut() {
                        match c.state() {
                            Ok(r) => state.radio = Some(r),
                            Err(e) => {
                                state.message = format!("Disconnected: {e:#}");
                                state.connected = false;
                                state.radio = None;
                                client = None;
                            }
                        }
                    }
                }
                let _ = updates.try_send(state.clone());
            }
        });
        Self {
            requests: tx,
            snapshots,
        }
    }
}
