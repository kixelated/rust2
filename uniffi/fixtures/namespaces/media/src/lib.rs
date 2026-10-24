/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

//! The upper of two layers: each type here shares its name with one in `net`, and holds or
//! refers to it.

use std::{collections::HashMap, sync::Arc};

use uniffi_namespaces_net as net;

uniffi::setup_scaffolding!("media");

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Frame {
    pub raw: net::Frame,
    pub keyframe: bool,
}

#[derive(Clone, Debug, PartialEq, uniffi::Enum)]
pub enum Priority {
    Net { priority: net::Priority },
    Realtime,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum GroupError {
    #[error("empty frame")]
    Empty,
    #[error("network: {source}")]
    Net { source: net::GroupError },
}

#[derive(Debug, uniffi::Object)]
pub struct GroupConsumer {
    group: Arc<net::GroupConsumer>,
}

#[uniffi::export]
impl GroupConsumer {
    #[uniffi::constructor]
    pub fn new(group: Arc<net::GroupConsumer>) -> Arc<Self> {
        Arc::new(Self { group })
    }

    pub fn group(&self) -> Arc<net::GroupConsumer> {
        self.group.clone()
    }

    pub fn priority(&self) -> Priority {
        match self.group.priority() {
            net::Priority::High => Priority::Realtime,
            priority => Priority::Net { priority },
        }
    }

    /// Reads and decodes the next frame.
    pub fn read_frame(&self) -> Result<Frame, GroupError> {
        let raw = self
            .group
            .read_frame()
            .map_err(|source| GroupError::Net { source })?;
        decode(raw)
    }

    /// Reads the next frame, throwing the `net` error as is.
    pub async fn next_frame(&self) -> Result<Frame, net::GroupError> {
        let raw = self.group.read_frame()?;
        Ok(Frame {
            keyframe: raw.payload.first() == Some(&1),
            raw,
        })
    }
}

/// Wraps a `net` frame, whose first byte marks a keyframe.
#[uniffi::export]
pub fn decode(raw: net::Frame) -> Result<Frame, GroupError> {
    match raw.payload.first() {
        None => Err(GroupError::Empty),
        Some(first) => Ok(Frame {
            keyframe: *first == 1,
            raw,
        }),
    }
}

/// Unwraps a `media` frame into the `net` frame it holds.
#[uniffi::export]
pub fn encode(frame: Frame) -> net::Frame {
    frame.raw
}

// Each of these holds a `net` type and its `media` namesake in the same container, so the
// helpers generated for both containers live side by side.

/// Decodes every `net` frame, skipping empty ones.
#[uniffi::export]
pub fn decode_all(raw: Vec<net::Frame>) -> Vec<Frame> {
    raw.into_iter().filter_map(|raw| decode(raw).ok()).collect()
}

/// The first keyframe, and the `net` frame of the last frame.
#[uniffi::export]
pub fn find_bounds(frames: Vec<Frame>) -> Bounds {
    Bounds {
        keyframe: frames.iter().find(|frame| frame.keyframe).cloned(),
        last: frames.last().map(|frame| frame.raw.clone()),
    }
}

#[derive(Clone, Debug, PartialEq, uniffi::Record)]
pub struct Bounds {
    pub keyframe: Option<Frame>,
    pub last: Option<net::Frame>,
}

/// Wraps each `net` group, keyed by its sequence, with the priorities both layers give it.
#[uniffi::export]
pub fn wrap_all(groups: Vec<Arc<net::GroupConsumer>>) -> Groups {
    let mut wrapped = Groups {
        groups: Vec::new(),
        net: HashMap::new(),
        media: HashMap::new(),
    };
    for group in groups {
        let sequence = group.sequence();
        wrapped.net.insert(sequence, group.priority());
        let group = GroupConsumer::new(group);
        wrapped.media.insert(sequence, group.priority());
        wrapped.groups.push(group);
    }
    wrapped
}

#[derive(uniffi::Record)]
pub struct Groups {
    pub groups: Vec<Arc<GroupConsumer>>,
    pub net: HashMap<u64, net::Priority>,
    pub media: HashMap<u64, Priority>,
}
