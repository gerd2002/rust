// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use container::Container;
use option::*;
use vec::OwnedVector;

pub struct WorkQueue<T> {
    priv queue: ~[T]
}

pub impl<T> WorkQueue<T> {
    fn new() -> WorkQueue<T> {
        WorkQueue {
            queue: ~[]
        }
    }

    fn push(&mut self, value: T) {
        self.queue.unshift(value)
    }

    fn pop(&mut self) -> Option<T> {
        if !self.queue.is_empty() {
            Some(self.queue.shift())
        } else {
            None
        }
    }

    fn steal(&mut self) -> Option<T> {
        if !self.queue.is_empty() {
            Some(self.queue.pop())
        } else {
            None
        }
    }

    fn is_empty(&self) -> bool {
        return self.queue.is_empty();
    }
}
