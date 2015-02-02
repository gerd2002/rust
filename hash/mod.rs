// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Generic hashing support.
//!
//! This module provides a generic way to compute the hash of a value. The
//! simplest way to make a type hashable is to use `#[derive(Hash)]`:
//!
//! # Examples
//!
//! ```rust
//! use std::hash::{hash, Hash, SipHasher};
//!
//! #[derive(Hash)]
//! struct Person {
//!     id: uint,
//!     name: String,
//!     phone: u64,
//! }
//!
//! let person1 = Person { id: 5, name: "Janet".to_string(), phone: 555_666_7777 };
//! let person2 = Person { id: 5, name: "Bob".to_string(), phone: 555_666_7777 };
//!
//! assert!(hash::<_, SipHasher>(&person1) != hash::<_, SipHasher>(&person2));
//! ```
//!
//! If you need more control over how a value is hashed, you need to implement
//! the trait `Hash`:
//!
//! ```rust
//! use std::hash::{hash, Hash, Hasher, Writer, SipHasher};
//!
//! struct Person {
//!     id: uint,
//!     name: String,
//!     phone: u64,
//! }
//!
//! impl<H: Hasher + Writer> Hash<H> for Person {
//!     fn hash(&self, state: &mut H) {
//!         self.id.hash(state);
//!         self.phone.hash(state);
//!     }
//! }
//!
//! let person1 = Person { id: 5, name: "Janet".to_string(), phone: 555_666_7777 };
//! let person2 = Person { id: 5, name: "Bob".to_string(), phone: 555_666_7777 };
//!
//! assert_eq!(hash::<_, SipHasher>(&person1), hash::<_, SipHasher>(&person2));
//! ```

#![unstable(feature = "hash",
            reason = "module was recently redesigned")]

use prelude::*;

use borrow::{Cow, ToOwned};
use default::Default;
use mem;
use num::Int;

pub use self::sip::SipHasher;

mod sip;

/// A hashable type.
///
/// The `H` type parameter is an abstract hash state that is used by the `Hash`
/// to compute the hash. Specific implementations of this trait may specialize
/// for particular instances of `H` in order to be able to optimize the hashing
/// behavior.
pub trait Hash<H: Hasher> {
    /// Feeds this value into the state given, updating the hasher as necessary.
    fn hash(&self, state: &mut H);
}

/// A trait which represents the ability to hash an arbitrary stream of bytes.
pub trait Hasher {
    /// Result type of one run of hashing generated by this hasher.
    type Output;

    /// Resets this hasher back to its initial state (as if it were just
    /// created).
    fn reset(&mut self);

    /// Completes a round of hashing, producing the output hash generated.
    fn finish(&self) -> Self::Output;
}

/// A common bound on the `Hasher` parameter to `Hash` implementations in order
/// to generically hash an aggregate.
#[unstable(feature = "hash",
           reason = "this trait will likely be replaced by io::Writer")]
#[allow(missing_docs)]
pub trait Writer {
    fn write(&mut self, bytes: &[u8]);
}

/// Hash a value with the default SipHasher algorithm (two initial keys of 0).
///
/// The specified value will be hashed with this hasher and then the resulting
/// hash will be returned.
pub fn hash<T: Hash<H>, H: Hasher + Default>(value: &T) -> H::Output {
    let mut h: H = Default::default();
    value.hash(&mut h);
    h.finish()
}

//////////////////////////////////////////////////////////////////////////////

macro_rules! impl_hash {
    ($ty:ident, $uty:ident) => {
        impl<S: Writer + Hasher> Hash<S> for $ty {
            #[inline]
            fn hash(&self, state: &mut S) {
                let a: [u8; ::$ty::BYTES] = unsafe {
                    mem::transmute((*self as $uty).to_le() as $ty)
                };
                state.write(a.as_slice())
            }
        }
    }
}

impl_hash! { u8, u8 }
impl_hash! { u16, u16 }
impl_hash! { u32, u32 }
impl_hash! { u64, u64 }
impl_hash! { uint, uint }
impl_hash! { i8, u8 }
impl_hash! { i16, u16 }
impl_hash! { i32, u32 }
impl_hash! { i64, u64 }
impl_hash! { int, uint }

impl<S: Writer + Hasher> Hash<S> for bool {
    #[inline]
    fn hash(&self, state: &mut S) {
        (*self as u8).hash(state);
    }
}

impl<S: Writer + Hasher> Hash<S> for char {
    #[inline]
    fn hash(&self, state: &mut S) {
        (*self as u32).hash(state);
    }
}

impl<S: Writer + Hasher> Hash<S> for str {
    #[inline]
    fn hash(&self, state: &mut S) {
        state.write(self.as_bytes());
        0xffu8.hash(state)
    }
}

macro_rules! impl_hash_tuple {
    () => (
        impl<S: Hasher> Hash<S> for () {
            #[inline]
            fn hash(&self, _state: &mut S) {}
        }
    );

    ( $($name:ident)+) => (
        impl<S: Hasher, $($name: Hash<S>),*> Hash<S> for ($($name,)*) {
            #[inline]
            #[allow(non_snake_case)]
            fn hash(&self, state: &mut S) {
                match *self {
                    ($(ref $name,)*) => {
                        $(
                            $name.hash(state);
                        )*
                    }
                }
            }
        }
    );
}

impl_hash_tuple! {}
impl_hash_tuple! { A }
impl_hash_tuple! { A B }
impl_hash_tuple! { A B C }
impl_hash_tuple! { A B C D }
impl_hash_tuple! { A B C D E }
impl_hash_tuple! { A B C D E F }
impl_hash_tuple! { A B C D E F G }
impl_hash_tuple! { A B C D E F G H }
impl_hash_tuple! { A B C D E F G H I }
impl_hash_tuple! { A B C D E F G H I J }
impl_hash_tuple! { A B C D E F G H I J K }
impl_hash_tuple! { A B C D E F G H I J K L }

impl<S: Writer + Hasher, T: Hash<S>> Hash<S> for [T] {
    #[inline]
    fn hash(&self, state: &mut S) {
        self.len().hash(state);
        for elt in self {
            elt.hash(state);
        }
    }
}


impl<'a, S: Hasher, T: ?Sized + Hash<S>> Hash<S> for &'a T {
    #[inline]
    fn hash(&self, state: &mut S) {
        (**self).hash(state);
    }
}

impl<'a, S: Hasher, T: ?Sized + Hash<S>> Hash<S> for &'a mut T {
    #[inline]
    fn hash(&self, state: &mut S) {
        (**self).hash(state);
    }
}

impl<S: Writer + Hasher, T> Hash<S> for *const T {
    #[inline]
    fn hash(&self, state: &mut S) {
        // NB: raw-pointer Hash does _not_ dereference
        // to the target; it just gives you the pointer-bytes.
        (*self as uint).hash(state);
    }
}

impl<S: Writer + Hasher, T> Hash<S> for *mut T {
    #[inline]
    fn hash(&self, state: &mut S) {
        // NB: raw-pointer Hash does _not_ dereference
        // to the target; it just gives you the pointer-bytes.
        (*self as uint).hash(state);
    }
}

impl<'a, T, B: ?Sized, S: Hasher> Hash<S> for Cow<'a, T, B>
    where B: Hash<S> + ToOwned<T>
{
    #[inline]
    fn hash(&self, state: &mut S) {
        Hash::hash(&**self, state)
    }
}
