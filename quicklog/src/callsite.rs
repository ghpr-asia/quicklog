//! Callsites represent the origin of a log line.
//!  
//! # What are Callsites?
//!
//! Every log macro is associated with a [`Callsite`]. A Callsite is
//! a static value which is responsible for the following:
//!
//! * Owning a sender onto the logging queue, which messages would be sent through

use crate::Sender;

/// A `Callsite` is a place where a macro is called, it owns its own
/// sender which can be used to send items onto the single sender queue
/// Sender is clonable amongst different threads as sender is part of mpsc
pub struct Callsite {
    /// Sender to send into a message queue owned by a Callsite
    pub sender: Sender,
}

/// Callsite is sendable amongst different threads as sender is send
unsafe impl Send for Callsite {}
/// Callsite is shareable amongst different threads as sender is sync
unsafe impl Sync for Callsite {}

impl Callsite {
    pub fn new(sender: Sender) -> Callsite {
        Callsite { sender }
    }
}
