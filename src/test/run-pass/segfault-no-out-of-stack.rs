// Copyright 2012-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![feature(libc)]

extern crate libc;

use std::process::{Command, ExitStatus};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 && args[1] == "segfault" {
        unsafe { *(0 as *mut isize) = 1 }; // trigger a segfault
    } else {
        let segfault = Command::new(&args[0]).arg("segfault").output().unwrap();
        let stderr = String::from_utf8_lossy(&segfault.stderr);
        let stdout = String::from_utf8_lossy(&segfault.stdout);
        println!("stdout: {}", stdout);
        println!("stderr: {}", stderr);
        println!("status: {}", segfault.status);
        assert!(!segfault.status.success());
        assert!(!stderr.contains("has overflowed its stack"));
    }
}