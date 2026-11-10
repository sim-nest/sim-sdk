// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Descriptive admission port.
//!
//! A host implements [`ProjectionAdmissionChannel`] and retains native
//! custody. This crate checks frames and identities. It does not open a
//! native channel.

use super::ProjectionAdmissionProtocolError;
use sim_lib_server::{ServerFrame, decode_transport_frame};
use std::{fs::File, io};

/// Device and inode numbers that name a kernel object on the wire.
///
/// These numbers are descriptive. They are not a retained file handle and
/// they do not grant admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KernelObjectIdentity {
    /// Filesystem device number.
    pub device: u64,
    /// Inode number.
    pub inode: u64,
}

/// One bounded transport message plus the descriptors that arrived with it.
#[derive(Debug)]
pub struct NativeDescriptorMessage {
    /// Exact transport-frame bytes.
    pub bytes: Vec<u8>,
    /// Descriptors that arrived with those bytes.
    pub descriptors: Vec<File>,
}

impl NativeDescriptorMessage {
    /// Accepts this message only when its descriptor count and decoded frame match.
    ///
    /// # Errors
    ///
    /// Returns an error when the descriptor count differs, the bytes are not
    /// one transport frame, or the decoded frame is not `expected`.
    pub fn bind_frame(
        self,
        expected: &ServerFrame,
        descriptor_count: usize,
    ) -> io::Result<Vec<File>> {
        if self.descriptors.len() != descriptor_count {
            return Err(invalid("admission message descriptor count differs"));
        }
        let actual = decode_transport_frame(&self.bytes).map_err(invalid_data)?;
        if &actual != expected {
            return Err(invalid("admission message differs from the expected frame"));
        }
        Ok(self.descriptors)
    }
}

/// Child-side admission channel. The implementor retains native custody.
pub trait ProjectionAdmissionChannel {
    /// Confirms the retained peer is still the creator of this channel.
    ///
    /// # Errors
    ///
    /// Returns an error when the creator or the channel is no longer current.
    fn revalidate(&self) -> Result<(), ProjectionAdmissionProtocolError>;

    /// Descriptive identity of the channel creator.
    fn creator_identity(&self) -> KernelObjectIdentity;

    /// Sends one exact frame.
    ///
    /// # Errors
    ///
    /// Returns an error when the frame cannot be sent on this channel.
    fn send_frame(&self, frame: &ServerFrame) -> Result<(), ProjectionAdmissionProtocolError>;

    /// Receives one message, or `None` when the peer has nothing ready.
    ///
    /// # Errors
    ///
    /// Returns an error when the receive fails.
    fn receive(&self) -> Result<Option<NativeDescriptorMessage>, ProjectionAdmissionProtocolError>;
}

fn invalid(detail: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, detail)
}

fn invalid_data(error: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_kernel::Symbol;
    use sim_lib_server::{FrameEnvelope, FrameKind, encode_transport_frame};

    fn frame(payload: &[u8]) -> ServerFrame {
        ServerFrame::new(
            Symbol::qualified("codec", "binary"),
            FrameKind::Response,
            FrameEnvelope::default(),
            payload.to_vec(),
        )
    }

    fn message(frame: &ServerFrame, descriptors: Vec<File>) -> NativeDescriptorMessage {
        NativeDescriptorMessage {
            bytes: encode_transport_frame(frame).expect("frame"),
            descriptors,
        }
    }

    #[test]
    fn bind_frame_accepts_the_exact_frame_and_count() {
        let expected = frame(b"exact");
        let files = message(&expected, Vec::new())
            .bind_frame(&expected, 0)
            .expect("exact frame");
        assert!(files.is_empty());
    }

    #[test]
    fn bind_frame_refuses_a_different_count_or_payload() {
        let expected = frame(b"exact");
        let extra = File::open("/dev/null").expect("null");
        assert!(
            message(&expected, vec![extra])
                .bind_frame(&expected, 0)
                .is_err()
        );
        let wrong = frame(b"other");
        assert!(
            message(&wrong, Vec::new())
                .bind_frame(&expected, 0)
                .is_err()
        );
    }
}
