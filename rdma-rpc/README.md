# RDMA-RPC

## Get started

Environment: ubuntu 20.04

1. Install dependencies:

    ```sh
    sudo apt install cmake llvm libibverbs-dev librdmacm-dev rdma-core
    ```

2. Because the provided rdma rust bindings are used in ubuntu 16, we need to hack something to make it work.
    In `rdma-shim/src/user/bindings.rs`, comment these lines out:

    ```rust
        pub mod ib_qp_type {
            pub use super::ibv_qp_type::*;

            pub const IB_QPT_RC: Type = IBV_QPT_RC;
            pub const IB_QPT_UC: Type = IBV_QPT_UC;
            pub const IB_QPT_UD: Type = IBV_QPT_UD;
            // pub const IB_QPT_XRC: Type = IBV_QPT_XRC;
            pub const IB_QPT_RAW_PACKET: Type = IBV_QPT_RAW_PACKET;
            // pub const IB_QPT_RAW_ETH: Type = IBV_QPT_RAW_ETH;
            pub const IB_QPT_XRC_SEND: Type = IBV_QPT_XRC_SEND;
            pub const IB_QPT_XRC_RECV: Type = IBV_QPT_XRC_RECV;
            // pub const IB_EXP_QP_TYPE_START: Type = IBV_EXP_QP_TYPE_START;
            // pub const IB_EXP_QPT_DC_INI: Type = IBV_EXP_QPT_DC_INI;
        }
    ```

    Hack the `libibverbs-dev` by replacing `/usr/include/infiniband/verbs.h` with the one in asssets:

    ```sh
    sudo cp assets/verbs.h /usr/include/infiniband/verbs.h
    ```

3. Get yourself a Soft-RoCE virtual nic by following this guide: <https://zhuanlan.zhihu.com/p/361740115>
4. Try the example!
    > note that you need to manually configure the `GID_IDX`, `qpn`, and `qkey` because different machine has different values. I hard coded my value for simplicity.
    * Run the server in one terminal: `cargo run --features user --example test_rdma_server`
    * Run the client in another terminal to send some message: `cargo run --features user --example test_rdma_client "hello rdma"`

## Road Map

### Transport

* [ ] establish connection between server and client
  * [ ] use tcp to exchange `lid`, `gid`, `qpn`, and `qkey` with server to establish rdma connection
  * [ ] manage resources(qp, ctx, cq, etc.)
* [ ] send and recv
* [ ] close connection, clean up resources

### Session

* [ ] design structures and mechanisms like tcp package to prevent data reorder, corruption, and loss

### Friendly user interface

* [ ] encapusulate internals(serialization, logging, etc.)
* [ ] Robust Error handling

### More advanced features

* [ ] timeout
* [ ] retry, idempotency
* [ ] support protobuf?
