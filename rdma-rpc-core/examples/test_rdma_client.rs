use std::env;

use clap::Parser;
use log::info;
use KRdmaKit::*;

#[derive(Parser, Debug)]
struct Args {
    msg: String,
}

const IB_PORT: u8 = 1;
const GID_IDX: usize = 2;
const BUF_SIZE: u64 = 1024;

fn main() {
    set_up_logger();
    let args = Args::parse();

    // create context
    let ctx = {
        let udriver = UDriver::create().expect("no device");
        let device = udriver.get_dev(0).expect("no device");
        info!("use device {}", device.name());
        device.open_context().expect("can't open context")
    };
    info!("Context: {:?}", ctx);

    // check the port attribute
    let port_attr = ctx.get_port_attr(IB_PORT).expect("query port error");
    info!("Port Attribute: {:?}", port_attr);

    // check the lid
    let lid = port_attr.lid;
    info!("LID: {:?}", lid);

    // check the gid
    let gid = ctx.query_gid(IB_PORT, GID_IDX).expect("query gid failed");
    info!("GID: {:?}", gid);

    // create a MR
    let mr = MemoryRegion::new(ctx.clone(), BUF_SIZE as usize).expect("failed to allocate MR");
    let buffer: &mut [u8] =
        unsafe { std::slice::from_raw_parts_mut(mr.get_virt_addr() as _, BUF_SIZE as usize) };
    info!("MR rkey and lkey: {:?} {:?}", mr.rkey(), mr.lkey());

    // create the client-side endpoint
    // TODO: you need to get these arguments from the server. I just use my default for simplicity.
    let endpoint = DatagramEndpoint::new(&ctx, 1, lid as u32, gid, 24, 73)
        .expect("UD endpoint creation fails");
    info!("sanity check UD endpoint: {:?}", endpoint);

    // create the client-side QP
    let qp = {
        let builder = QueuePairBuilder::new(&ctx);
        let qp = builder
            .build_ud()
            .expect("failed to build UD QP")
            .bring_up_ud()
            .expect("failed to bring up UD QP");
        info!("QP status: {:?}", qp.status());
        qp
    };

    // very unsafe to construct a send message
    let msg = args.msg.as_bytes();
    buffer[..msg.len()].copy_from_slice(msg);

    // really send the message
    qp.post_datagram(&endpoint, &mr, 0..(msg.len() as u64), 73, true)
        .expect("failed to post message");

    // sleep 1 second to ensure the message has arrived
    std::thread::sleep(std::time::Duration::from_millis(1000));
}

fn set_up_logger() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    env_logger::builder()
        .format_target(false)
        .format_timestamp(None)
        .init();
}
