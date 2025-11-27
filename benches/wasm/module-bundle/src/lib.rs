use core::slice;

use bench::{assert_big_input, assert_small_input};

unsafe extern "C" {
    fn input(ptr: u64);
    fn input_len() -> u64;
}

#[unsafe(no_mangle)]
pub fn noop() {}

#[unsafe(no_mangle)]
pub fn run_small() {
    let n = unsafe { input_len() };
    let buf = vec![0; n as _];
    let ptr = buf.as_ptr() as u64;
    unsafe { input(ptr) };
    let v = serde_json::from_slice(&buf).unwrap();
    assert_small_input(v)
}

#[unsafe(no_mangle)]
pub fn run_small_bytes(ptr: *const u8, len: usize) {
    let v = serde_json::from_slice(unsafe { slice::from_raw_parts(ptr, len) }).unwrap();
    assert_small_input(v)
}

#[unsafe(no_mangle)]
pub fn run_big() {
    let n = unsafe { input_len() };
    let buf = vec![0; n as _];
    let ptr = buf.as_ptr() as u64;
    unsafe { input(ptr) };
    let v = serde_json::from_slice(&buf).unwrap();
    assert_big_input(v)
}

#[unsafe(no_mangle)]
pub fn run_big_bytes(ptr: *const u8, len: usize) {
    let v = serde_json::from_slice(unsafe { slice::from_raw_parts(ptr, len) }).unwrap();
    assert_big_input(v)
}
