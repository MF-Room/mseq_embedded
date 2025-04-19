use core::mem::MaybeUninit;
use embedded_alloc::LlffHeap as Heap;

const HEAP_SIZE: usize = 1024;

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub fn allocator_init() {
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_mut_ptr() as usize, HEAP_SIZE) }
}
