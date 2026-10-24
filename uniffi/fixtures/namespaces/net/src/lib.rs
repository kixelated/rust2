/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The lower of two layers whose types share their names with the upper one, `media`.

use std::sync::{Arc, Mutex};

uniffi::setup_scaffolding!("net");

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Frame {
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, uniffi::Enum)]
pub enum Priority {
    Low,
    High,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GroupError {
    #[error("group closed with code {code}")]
    Closed { code: u32 },
}

#[derive(Debug, uniffi::Object)]
pub struct GroupConsumer {
    sequence: u64,
    priority: Priority,
    frames: Mutex<Vec<Frame>>,
}

#[uniffi::export]
impl GroupConsumer {
    #[uniffi::constructor]
    pub fn new(sequence: u64, priority: Priority, frames: Vec<Frame>) -> Arc<Self> {
        Arc::new(Self {
            sequence,
            priority,
            frames: Mutex::new(frames),
        })
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// Takes the next frame, or fails with code 7 once there are none left.
    pub fn read_frame(&self) -> Result<Frame, GroupError> {
        let mut frames = self.frames.lock().unwrap();
        if frames.is_empty() {
            return Err(GroupError::Closed { code: 7 });
        }
        Ok(frames.remove(0))
    }
}
