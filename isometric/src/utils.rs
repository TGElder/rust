use std::cmp::Ordering;
use std::ffi::CString;

#[allow(unused_mut)]
pub fn create_whitespace_cstring_with_len(length: usize) -> CString {
    let mut buffer: Vec<u8> = vec![b' '; length + 1];
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub fn unsafe_ordering(a: &&f32, b: &&f32) -> Ordering {
    a.partial_cmp(b).unwrap()
}
