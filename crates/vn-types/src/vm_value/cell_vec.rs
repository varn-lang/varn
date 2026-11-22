use std::fmt;
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

const OWNED: usize = 1 << (usize::BITS - 1);

#[repr(C)]
pub struct CellVec<T: Copy> {
    ptr: NonNull<T>,
    len: usize,
    cap_and_owner: usize,
}

unsafe impl<T: Copy + Send> Send for CellVec<T> {}
unsafe impl<T: Copy + Sync> Sync for CellVec<T> {}

impl<T: Copy> CellVec<T> {
    pub const PTR_OFF: usize = std::mem::offset_of!(CellVec<T>, ptr);
    pub const LEN_OFF: usize = std::mem::offset_of!(CellVec<T>, len);

    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap_and_owner: 0,
        }
    }

    #[inline(always)]
    pub unsafe fn inline(ptr: *mut T, len: usize, cap: usize) -> Self {
        Self {
            ptr: unsafe { NonNull::new_unchecked(ptr) },
            len,
            cap_and_owner: cap,
        }
    }

    pub fn from_vec(vec: Vec<T>) -> Self {
        let mut vec = std::mem::ManuallyDrop::new(vec);
        if vec.capacity() == 0 {
            return Self::new();
        }
        Self {
            ptr: unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) },
            len: vec.len(),
            cap_and_owner: vec.capacity() | OWNED,
        }
    }

    pub fn into_vec(self) -> Vec<T> {
        let this = std::mem::ManuallyDrop::new(self);
        if this.is_owned() {
            unsafe { Vec::from_raw_parts(this.ptr.as_ptr(), this.len, this.capacity()) }
        } else {
            this.as_slice().to_vec()
        }
    }

    #[inline(always)]
    fn is_owned(&self) -> bool {
        self.cap_and_owner & OWNED != 0
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.cap_and_owner & !OWNED
    }

    #[inline(always)]
    pub fn heap_capacity(&self) -> usize {
        if self.is_owned() {
            self.capacity()
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }

    #[inline(always)]
    pub fn push(&mut self, value: T) {
        if self.len == self.capacity() {
            self.grow(1);
        }
        unsafe { self.ptr.as_ptr().add(self.len).write(value) };
        self.len += 1;
    }

    #[inline(always)]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(unsafe { self.ptr.as_ptr().add(self.len).read() })
    }

    pub fn extend_from_slice(&mut self, items: &[T]) {
        self.reserve(items.len());
        unsafe {
            std::ptr::copy_nonoverlapping(
                items.as_ptr(),
                self.ptr.as_ptr().add(self.len),
                items.len(),
            );
        }
        self.len += items.len();
    }

    pub fn reserve(&mut self, additional: usize) {
        if self.capacity() - self.len < additional {
            self.grow(additional);
        }
    }

    pub fn truncate(&mut self, len: usize) {
        self.len = self.len.min(len);
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn edit<R>(&mut self, f: impl FnOnce(&mut Vec<T>) -> R) -> R {
        let mut vec = std::mem::take(self).into_vec();
        let out = f(&mut vec);
        *self = Self::from_vec(vec);
        out
    }

    #[cold]
    fn grow(&mut self, additional: usize) {
        let needed = self.len + additional;
        let new_cap = needed.max(self.capacity() * 2).max(4);
        let mut fresh: Vec<T> = Vec::with_capacity(new_cap);
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr.as_ptr(), fresh.as_mut_ptr(), self.len);
            fresh.set_len(self.len);
        }
        let old = std::mem::replace(self, Self::from_vec(fresh));
        drop(old);
    }
}

impl<T: Copy> Drop for CellVec<T> {
    fn drop(&mut self) {
        if self.is_owned() {
            drop(unsafe { Vec::from_raw_parts(self.ptr.as_ptr(), 0, self.capacity()) });
        }
    }
}

impl<T: Copy> Default for CellVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy> Deref for CellVec<T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: Copy> DerefMut for CellVec<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Copy> Clone for CellVec<T> {
    fn clone(&self) -> Self {
        Self::from_vec(self.as_slice().to_vec())
    }
}

impl<T: Copy + fmt::Debug> fmt::Debug for CellVec<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl<T: Copy> From<Vec<T>> for CellVec<T> {
    fn from(vec: Vec<T>) -> Self {
        Self::from_vec(vec)
    }
}

impl<T: Copy> FromIterator<T> for CellVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

impl<'a, T: Copy> IntoIterator for &'a CellVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.as_slice().iter()
    }
}
