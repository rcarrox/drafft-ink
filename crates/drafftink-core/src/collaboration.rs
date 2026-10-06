//! Collaboration-disabled manager used by the personal/local build.

use crate::canvas::CanvasDocument;
use crate::shapes::Shape;

#[derive(Default)]
pub struct DummyCrdt;

impl DummyCrdt {
    pub fn add_shape(&mut self, _shape: &Shape) -> Result<(), String> { Ok(()) }
    pub fn remove_shape(&mut self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn bring_to_front(&mut self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn send_to_back(&mut self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn bring_forward(&mut self, _id: &str) -> Result<(), String> { Ok(()) }
    pub fn send_backward(&mut self, _id: &str) -> Result<(), String> { Ok(()) }
}

pub struct CollaborationManager {
    dummy: DummyCrdt,
}

impl CollaborationManager {
    pub fn new() -> Self {
        Self { dummy: DummyCrdt }
    }

    pub fn is_in_room(&self) -> bool { false }
    pub fn enable(&mut self) {}
    pub fn disable(&mut self) {}
    pub fn set_room(&mut self, _room: Option<String>) {}
    pub fn join_room(&mut self, _room: &str) {}
    pub fn leave_room(&mut self) {}
    pub fn set_user_info(&mut self, _name: String, _color: String) {}
    pub fn set_cursor(&mut self, _x: f64, _y: f64) {}
    pub fn sync_to_crdt(&mut self, _doc: &CanvasDocument) {}
    pub fn sync_from_crdt(&self, _doc: &mut CanvasDocument) {}
    pub fn broadcast_sync(&mut self) {}
    pub fn import_updates(&mut self, _bytes: &[u8]) -> bool { false }
    pub fn has_outgoing(&self) -> bool { false }
    pub fn take_outgoing(&mut self) -> Vec<String> { Vec::new() }
    pub fn crdt_mut(&mut self) -> &mut DummyCrdt { &mut self.dummy }
}

impl Default for CollaborationManager {
    fn default() -> Self { Self::new() }
}
