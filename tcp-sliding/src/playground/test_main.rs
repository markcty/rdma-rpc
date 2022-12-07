#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MyStruct {
    foo: u16,
    bar: u8,
    data: [u8; 16],
}
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
pub struct Row {
    id: u32,
    username: [u8; 3],
    email: [u8; 5],
}
pub struct Frame {
    id: u16,
    data: [u8; 255],
}
unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}
pub unsafe fn serialize_frame<T: Sized>(src: &T) -> &[u8] {
    ::std::slice::from_raw_parts((src as *const T) as *const u8, ::std::mem::size_of::<T>())
}
pub unsafe fn deserialize_frame(src: Vec<u8>) -> Frame {
    std::ptr::read(src.as_ptr() as *const _)
}
pub unsafe fn serialize_row<T: Sized>(src: &T) -> &[u8] {
    ::std::slice::from_raw_parts((src as *const T) as *const u8, ::std::mem::size_of::<T>())
}

pub unsafe fn deserialize_row(src: &[u8]) -> Row {
    std::ptr::read(src.as_ptr() as *const _)
}
pub fn test_from_stack_overflow() {
    let my_str = MyStruct {
        foo: 12,
        bar: 21,
        data: [1; 16],
    };
    let bytes: &[u8] = unsafe { any_as_u8_slice(&my_str) };
    println!("{:?}", bytes);
    let (head, body, _tail) = unsafe { bytes.align_to::<MyStruct>() };
    // let (head, body, _tail) = unsafe { v.align_to::<MyStruct>() };
    assert!(head.is_empty(), "Data was not aligned");
    let my_struct = &body[0];

    println!("{:?}", my_struct);
}
pub fn test_main() {
    let my_row = Row {
        id: 12u32,
        username: [1; 3],
        email: [8; 5],
    };
    let row_by: &[u8] = unsafe { serialize_row(&my_row) };
    println!("{:?}, len = {:?}", row_by, row_by.len());
    let re_get_row = unsafe { deserialize_row(row_by) };
    println!("{:?}", re_get_row);
}
