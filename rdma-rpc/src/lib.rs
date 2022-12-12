extern crate alloc;

use std::{
    io::{Read, Write},
    marker::PhantomData,
    net::{SocketAddrV4, TcpListener},
    thread,
};

use alloc::{collections::BTreeMap, sync::Arc};
use rdma_rpc_core::{
    client_stub::ClientStub,
    server_stub::{RpcHandler, ServerStub},
    session::Session,
    transport::Transport,
};
use serde::{de::DeserializeOwned, Serialize};
use spin::mutex::Mutex;

pub struct Server<T, R> {
    addr: SocketAddrV4,
    dev: String,
    ib_port: u8,
    handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
    sessions: Arc<Mutex<BTreeMap<u64, Session>>>,
}

#[derive(Debug)]
pub struct Error;

impl<T, R> Server<T, R>
where
    T: DeserializeOwned + 'static,
    R: Serialize + 'static,
{
    pub fn new(
        dev: &str,
        ib_port: u8,
        addr: SocketAddrV4,
        handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
    ) -> Result<Server<T, R>, Error> {
        todo!();
    }

    pub fn serve(self) {
        let listener = TcpListener::bind(self.addr).unwrap();

        // run server stub
        let server_stub = ServerStub::new(
            self.dev.as_str(),
            self.ib_port,
            Arc::clone(&self.sessions),
            Arc::clone(&self.handler),
        )
        .unwrap();
        thread::spawn(move || server_stub.serve());

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    // create a new session
                    let session_id = rand::random();

                    // receive QPInfo from stream
                    let mut buf: Vec<u8> = Vec::new();
                    stream
                        .read_to_end(&mut buf)
                        .expect("failed to read client info");
                    let client_qp_info =
                        bincode::deserialize(&buf).expect("failed to deserialize qp info"); // TODO: handle error

                    // create the transport

                    let transport = Transport::new(
                        server_stub.qp(),
                        client_qp_info,
                        server_stub.ctx(),
                        server_stub.mr(),
                    )
                    .unwrap(); // TODO: handle error

                    // send self qp_info
                    let server_qp_info = transport.self_qp_info();
                    let data = bincode::serialize(&server_qp_info).unwrap();
                    let res = stream.write_all(&data);
                    if res.is_err() {
                        // TODO: handle error
                    }

                    // create session
                    let session = Session::new(session_id, transport);

                    // add the session to the server stub
                    assert!(
                        self.sessions.lock().insert(session_id, session).is_none(),
                        "duplicated session {session_id}",
                    );

                    // TODO: handle client close?
                }
                Err(e) => { /* connection failed */ }
            }
        }
    }
}

pub struct Client<T, R> {
    client_stub: ClientStub,
    phantom_t: PhantomData<T>,
    phantom_r: PhantomData<R>,
}

impl<T, R> Client<T, R>
where
    T: Serialize + 'static,
    R: DeserializeOwned + 'static,
{
    pub fn new() -> Self {
        todo!()
    }

    pub fn send(&self, args: T) -> Result<R, Error> {
        Ok(self.client_stub.sync_call(args).unwrap()) // TODO: handle error
    }
}
