mod config;
mod tcp;
use crate::tcp::types::serialize_any;
use bytes::{BufMut, BytesMut};
use crossbeam::channel::bounded;
use std::thread;
use std::time;
use std::time::Duration;
use tcp::client;
use tcp::server;
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Row {
    bar: u32,
    content: &'static [u8],
}
fn ser_test() {
    let data = &[3u8; 100];
    let content: &[u8] = data[0..2].try_into().unwrap();
    println!("{:?}", content);
    let row = Row {
        bar: 12u32,
        content,
    };
    println!("{:?}", row);
    let ser_get = unsafe { serialize_any(&row) };
    println!("{:?}", ser_get);
}
fn build_up_bytesmut(data: &mut BytesMut) {
    let data_1 = [1u8; 4];
    data.put(&data_1[..]);
    let data_2 = [3u8; 4];
    data.put(&data_2[..]);
    return;
}
fn bytesmut_method_test() {
    let mut buf = BytesMut::with_capacity(64);

    buf.put(&[1u8][..]);
    // buf.put(b'e');
    // buf.put("llo");

    assert_eq!(&buf[..], &[1u8][..]);

    // Freeze the buffer so that it can be shared
    let a = buf.freeze();

    // This does not allocate, instead `b` points to the same memory.
    let b = a.clone();

    assert_eq!(&a[..], &[1u8][..]);
    assert_eq!(&b[..], &[1u8][..]);
}
fn bytemute_test() {
    let mut buf = BytesMut::with_capacity(64);
    build_up_bytesmut(&mut buf);
    let a = buf.freeze();
    let b = a.clone();
    println!("a = {:?}", &a[..]);
}
fn server_client_test() {
    server::run_server_background();
    thread::sleep(time::Duration::from_secs(1));
    // client::run_client();
    client::client_send_test();
}

fn lock_test() {
    let (s, r) = bounded(1);
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(3)); //假设线程使用了3秒
        s.send(()).unwrap();
    });
    //2秒超时
    if let Ok(msg) = r.recv_timeout(Duration::from_secs(2)) {
        println!("Hello, world!, got msg = {:?}", msg);
    } else {
        println!("Time out");
    };
}
fn data_insert_test() {
    let mut total_data = BytesMut::with_capacity(8);
    let mut data = [0u8; 4];
    data[2] = 1;
    total_data.put(&data[..]);
    println!("total data = {:?}", &total_data[..]);
    data[2] = 8;
    total_data.put(&data[..]);
    println!("total data = {:?}", &total_data[..]);
}
fn main() {
    println!("hello world");
    // lock_test();
    // data_insert_test()
    server_client_test();
    // ser_test();
    // bytemute_test();
}
