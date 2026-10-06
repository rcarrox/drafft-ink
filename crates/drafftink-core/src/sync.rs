//! Collaboration-disabled synchronization stubs.
//!
//! The personal/local build keeps the application API stable while compiling
//! out WebSocket and CRDT dependencies. Collaboration controls are hidden by
//! the app, so these types are intentionally inert.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AwarenessState {
    pub cursor: Option<CursorPosition>,
    pub user: Option<UserInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone)]
pub enum SyncEvent {
    Connected,
    Disconnected,
    JoinedRoom {
        room: String,
        peer_count: usize,
        initial_sync: Option<Vec<u8>>,
    },
    PeerJoined { peer_id: String },
    PeerLeft { peer_id: String },
    SyncReceived { from: String, data: Vec<u8> },
    AwarenessReceived {
        from: String,
        peer_id: u64,
        state: AwarenessState,
    },
    Error { message: String },
}

macro_rules! disabled_socket {
    ($name:ident) => {
        pub struct $name {
            state: ConnectionState,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    state: ConnectionState::Disconnected,
                }
            }

            pub fn connect(&mut self, _url: &str) -> Result<(), String> {
                self.state = ConnectionState::Error;
                Err("Collaboration is disabled in this build".to_string())
            }

            pub fn disconnect(&mut self) {
                self.state = ConnectionState::Disconnected;
            }

            pub fn send(&self, _msg: &str) -> Result<(), String> {
                Err("Collaboration is disabled in this build".to_string())
            }

            pub fn poll_events(&mut self) -> Vec<SyncEvent> {
                Vec::new()
            }

            pub fn state(&self) -> ConnectionState {
                self.state
            }

            pub fn is_connected(&self) -> bool {
                false
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

disabled_socket!(WasmWebSocket);
disabled_socket!(NativeWebSocket);

#[cfg(target_arch = "wasm32")]
pub type PlatformWebSocket = WasmWebSocket;
#[cfg(not(target_arch = "wasm32"))]
pub type PlatformWebSocket = NativeWebSocket;
