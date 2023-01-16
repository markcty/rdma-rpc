pub(crate) fn sleep_millis(duration: u32) {
    unsafe {
        libc::usleep(1000 * duration);
    }
}

/// Utils for tests
#[cfg(test)]
pub(crate) mod tests {
    use alloc::{sync::Arc, vec::Vec};
    use core::iter;

    use libc::time_t;
    use KRdmaKit::{context::Context, random, services_user, QueuePair, QueuePairBuilder, UDriver};

    use crate::{messages::QPInfo, transport::Transport};

    pub(crate) fn new_test_context() -> Arc<Context> {
        let udriver = UDriver::create().unwrap();
        let device = udriver.devices().get(0).unwrap();
        device.open_context().unwrap()
    }

    pub(crate) fn new_test_qp(context: &Arc<Context>) -> Arc<QueuePair> {
        QueuePairBuilder::new(context)
            .build_ud()
            .unwrap()
            .bring_up_ud()
            .unwrap()
    }

    pub(crate) fn new_two_transport() -> (Transport, Transport) {
        let context = new_test_context();
        let qp1 = new_test_qp(&context);
        let qp_info1 = QPInfo {
            lid: qp1.lid().unwrap(),
            gid: services_user::ibv_gid_wrapper::from(qp1.gid().unwrap()),
            qp_num: qp1.qp_num(),
            qkey: qp1.qkey(),
        };
        let qp2 = new_test_qp(&context);
        let qp_info2 = QPInfo {
            lid: qp2.lid().unwrap(),
            gid: services_user::ibv_gid_wrapper::from(qp2.gid().unwrap()),
            qp_num: qp2.qp_num(),
            qkey: qp2.qkey(),
        };

        let tp1 = Transport::new_with_qp(qp1, Arc::clone(&context), qp_info2, 1).unwrap();
        let tp2 = Transport::new_with_qp(qp2, Arc::clone(&context), qp_info1, 1).unwrap();

        (tp1, tp2)
    }

    pub(crate) fn new_random_data(n_bytes: usize) -> Vec<u8> {
        let mut rng = random::FastRandom::new(unsafe {
            let mut t: time_t = 0;
            libc::time(&mut t as _) as _
        });
        iter::repeat_with(|| rng.get_next().to_be_bytes()[0])
            .take(n_bytes)
            .collect()
    }
}
