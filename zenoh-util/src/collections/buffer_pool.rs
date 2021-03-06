//
// Copyright (c) 2017, 2020 ADLINK Technology Inc.
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   ADLINK zenoh team, <zenoh@adlink-labs.tech>
//
use super::FifoQueue;
use async_std::sync::{Arc, Weak};
use async_std::task;
use std::fmt;
use std::ops::{Deref, DerefMut, Drop};

/// Provides a pool of pre-allocated buffers that are automaticlaly reinserted into
/// the pool when dropped.
pub struct RecyclingBufferPool {
    inner: Arc<FifoQueue<Vec<u8>>>,
}

impl RecyclingBufferPool {
    pub fn new(num: usize, size: usize) -> RecyclingBufferPool {
        let inner: Arc<FifoQueue<Vec<u8>>> = Arc::new(FifoQueue::new(num));
        for _ in 0..num {
            let buffer = vec![0u8; size];
            inner.try_push(buffer);
        }
        RecyclingBufferPool { inner }
    }

    pub async fn pull(&self) -> RecyclingBuffer {
        let buffer = self.inner.pull().await;
        RecyclingBuffer::new(buffer, Arc::downgrade(&self.inner))
    }
}

#[derive(Clone)]
pub struct RecyclingBuffer {
    buffer: Option<Vec<u8>>,
    pool: Weak<FifoQueue<Vec<u8>>>,
}

impl RecyclingBuffer {
    pub fn new(buffer: Vec<u8>, pool: Weak<FifoQueue<Vec<u8>>>) -> RecyclingBuffer {
        RecyclingBuffer {
            buffer: Some(buffer),
            pool,
        }
    }
}

impl Deref for RecyclingBuffer {
    type Target = Vec<u8>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.buffer.as_ref().unwrap()
    }
}

impl DerefMut for RecyclingBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buffer.as_mut().unwrap()
    }
}

impl Drop for RecyclingBuffer {
    fn drop(&mut self) {
        if let Some(pool) = self.pool.upgrade() {
            let buffer = self.buffer.take().unwrap();
            task::block_on(pool.push(buffer));
        }
    }
}

impl fmt::Debug for RecyclingBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Buffer").field("inner", &self).finish()
    }
}
