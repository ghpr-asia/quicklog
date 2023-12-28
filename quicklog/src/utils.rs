#![allow(unused)]

#[inline]
#[cold]
fn cold() {}

#[inline]
pub(crate) fn likely(b: bool) -> bool {
    if !b {
        cold()
    }
    b
}

#[inline]
pub(crate) fn unlikely(b: bool) -> bool {
    if b {
        cold()
    }
    b
}

#[inline(always)]
pub(crate) fn any_as_bytes<T: Sized>(a: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>()) }
}
