// Copyright 2012 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

/**
Clonable types are copied with the clone method
*/
pub trait Clone {
    fn clone(&self) -> Self;
}

impl Clone for () {
    #[inline(always)]
    fn clone(&self) -> () { () }
}

impl<T:Clone> Clone for ~T {
    #[inline(always)]
    fn clone(&self) -> ~T { ~(**self).clone() }
}

macro_rules! clone_impl(
    ($t:ty) => {
        impl Clone for $t {
            #[inline(always)]
            fn clone(&self) -> $t { *self }
        }
    }
)

clone_impl!(int)
clone_impl!(i8)
clone_impl!(i16)
clone_impl!(i32)
clone_impl!(i64)

clone_impl!(uint)
clone_impl!(u8)
clone_impl!(u16)
clone_impl!(u32)
clone_impl!(u64)

clone_impl!(float)
clone_impl!(f32)
clone_impl!(f64)

clone_impl!(bool)
clone_impl!(char)
