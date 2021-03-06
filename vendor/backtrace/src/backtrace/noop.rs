//! Empty implementation of unwinding used when no other implementation is
//! appropriate.

use types::c_void;

#[inline(always)]
pub fn trace(_cb: &mut FnMut(&super::Frame) -> bool) {}

#[derive(Clone)]
pub struct Frame;

impl Frame {
    pub fn ip(&self) -> *mut c_void {
        0 as *mut _
    }

    pub fn symbol_address(&self) -> *mut c_void {
        0 as *mut _
    }
}
