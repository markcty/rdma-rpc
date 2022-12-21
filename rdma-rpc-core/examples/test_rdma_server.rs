use std::env;

use KRdmaKit::*;

const IB_PORT: u8 = 1;
const GID_IDX: usize = 2;
const BUF_SIZE: u64 = 1024;
const UD_DATA_OFFSET: usize = 40; // For a UD message, the first 40 bytes are reserved for GRH

fn main() {
    set_up_logger();

    // create context
    let ctx = {
        let udriver = UDriver::create().expect("no device");
        let device = udriver.get_dev(0).expect("no device");
        println!("use device {}", device.name());
        device.open_context().expect("can't open context")
    };
    println!("Context: {:?}", ctx);

    // check the port attribute
    let port_attr = ctx.get_port_attr(IB_PORT).expect("query port error");
    println!("Port Attribute: {:?}", port_attr);

    // check the lid
    let lid = port_attr.lid;
    println!("LID: {:?}", lid);

    // check the gid
    let gid = ctx.query_gid(IB_PORT, GID_IDX).expect("query gid failed");
    println!("GID: {:?}", gid);

    // create a MR
    let mr = MemoryRegion::new(ctx.clone(), BUF_SIZE as usize).expect("failed to allocate MR");
    println!("MR rkey and lkey: {:?} {:?}", mr.rkey(), mr.lkey());
    let wr_id = mr.get_virt_addr(); // just use va of mr as wr_id
    let data_va = (mr.get_virt_addr() as usize + UD_DATA_OFFSET) as *mut u8;

    // create qp
    let qp = {
        let builder = QueuePairBuilder::new(&ctx);
        let qp = builder
            .build_ud()
            .expect("failed to build UD QP")
            .bring_up_ud()
            .expect("failed to bring up UD QP");
        println!("QP status: {:?}", qp.status());
        qp
    };
    println!("QP num: {:?}, qkey: {:?}", qp.qp_num(), qp.qkey());

    qp.post_recv(&mr, 0..BUF_SIZE, wr_id)
        .expect("failed to register recv buffer");

    // try to receive
    loop {
        // try to poll every 50ms
        std::thread::sleep(std::time::Duration::from_millis(50));

        // poll one wc every time
        let mut wcs = [Default::default()];
        let res = qp.poll_recv_cq(&mut wcs).expect("failed to poll recv CQ");

        if res.is_empty() {
            continue;
        }

        // decode the data
        let msg_sz = res[0].byte_len as usize - UD_DATA_OFFSET;
        let msg = std::str::from_utf8(unsafe { std::slice::from_raw_parts(data_va, msg_sz) })
            .expect("failed to decode msg");
        println!("msg received: {msg}");

        // recv again
        qp.post_recv(&mr, 0..BUF_SIZE, wr_id)
            .expect("failed to register recv buffer");
    }
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
