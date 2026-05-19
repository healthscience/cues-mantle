use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::ptr;

pub const SUBSTRATE_SIZE: usize = 4096;
pub const LEDGER_SIZE: usize = 1024;
pub const ALIGNMENT: usize = 16;

pub struct SubstrateBuffer {
    pub ptr: *mut u8,
    pub size: usize,
}

impl SubstrateBuffer {
    pub fn new(size: usize) -> Self {
        let layout = Layout::from_size_align(size, ALIGNMENT).unwrap();
        let ptr = unsafe { alloc_zeroed(layout) };
        Self { ptr, size }
    }

    pub fn as_slice(&self) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(self.ptr as *const f32, self.size / 4)
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr as *mut f32, self.size / 4)
        }
    }
}

impl Drop for SubstrateBuffer {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.size, ALIGNMENT).unwrap();
        unsafe {
            dealloc(self.ptr, layout);
        }
    }
}

pub struct MemorySubstrate {
    pub main: SubstrateBuffer,
    pub ledger: SubstrateBuffer,
}

impl MemorySubstrate {
    pub fn new() -> Self {
        Self {
            main: SubstrateBuffer::new(SUBSTRATE_SIZE),
            ledger: SubstrateBuffer::new(LEDGER_SIZE),
        }
    }
}

unsafe impl Send for MemorySubstrate {}
unsafe impl Sync for MemorySubstrate {}
