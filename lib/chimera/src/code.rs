use crate::pool::{AllocErr, AllocId, AllocMetadata, ItemAlloc, PagePool};
use std::{
    alloc::{alloc, Layout},
    any::Any,
    mem::{align_of, size_of},
    ptr::NonNull,
    slice,
};
use wasmer_runtime_core::types::LocalFuncIndex;

struct CodeAlloc {
    code_size: u32,
    keep_alive: Box<dyn Any>,
}

unsafe impl ItemAlloc for CodeAlloc {
    type Output = Code;

    fn metadata(&self) -> AllocMetadata {
        AllocMetadata {
            size: size_of::<Code>() + self.code_size as usize,
            executable: true,
        }
    }

    unsafe fn in_place(self, header: *mut Code) {
        (&mut (*header).keep_alive as *mut Box<dyn Any>).write(self.keep_alive);
        (&mut (*header).call_offsets as *mut Box<[CallOffset]>).write(Box::new([]));
        (*header).code_size = self.code_size;
    }
}

#[repr(C)]
pub struct CallOffset {
    pub func_index: LocalFuncIndex,
    pub offset: u32,
}

#[repr(C)]
pub struct Code {
    pub keep_alive: Box<dyn Any>,
    pub call_offsets: Box<[CallOffset]>,
    code_size: u32,
    code: [u8; 0],
}

impl Code {
    pub fn new(
        pool: &PagePool,
        code_size: u32,
        keep_alive: impl Any,
    ) -> Result<AllocId<Code>, AllocErr> {
        let code_alloc = CodeAlloc {
            keep_alive: Box::new(keep_alive),
            code_size,
        };

        pool.alloc(code_alloc)
    }

    pub fn code_ptr(&self) -> NonNull<u8> {
        unsafe { NonNull::new_unchecked(self.code.as_ptr() as *mut u8) }
    }

    pub fn code_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.code.as_mut_ptr(), self.code_size as usize) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alloc() {
        let pool = PagePool::new();
        let _code = Code::new(&pool, 16, ()).unwrap();
    }

    #[test]
    fn test_exec() {
        fn assemble_jmp(address: u64) -> [u8; 16] {
            let mut buf = [0; 16];

            buf[..2].copy_from_slice(&[0x48, 0xb8]);
            buf[2..10].copy_from_slice(&address.to_le_bytes());
            buf[10..12].copy_from_slice(&[0xff, 0xe0]);

            buf
        }

        unsafe fn callable() -> usize {
            42
        }

        let pool = PagePool::new();
        let mut code_id = Code::new(&pool, 16, ()).unwrap();

        let mut code = pool.get_mut(&mut code_id);

        code.code_mut()
            .copy_from_slice(&assemble_jmp(callable as u64));

        let result = unsafe {
            let func_ptr: unsafe fn() -> usize = std::mem::transmute(code.code_ptr());
            func_ptr()
        };

        assert_eq!(result, 42);
    }
}
