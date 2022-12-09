extern crate alloc;

use std::{
    marker::PhantomData,
    net::{SocketAddrV4, TcpListener, TcpStream},
    thread,
};

use alloc::{collections::BTreeMap, sync::Arc};
use rdma_rpc_core::{
    client_stub::ClientStub,
    messages::QPInfo,
    server_stub::{RpcHandler, ServerStub},
    session::Session,
    transport::Transport,
};
use serde::{de::DeserializeOwned, Serialize};
use spin::mutex::Mutex;
use KRdmaKit::context::Context;

pub struct Server<T, R> {
    addr: SocketAddrV4,
    dev: String,
    ib_port: u8,
    context: Arc<Context>,
    handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
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

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => self.handle_client(stream),
                Err(e) => { /* connection failed */ }
            }
        }
    }

    pub fn handle_client(&self, stream: TcpStream) {
        // create a new session
        let session_id = rand::random();

        // receive QPInfo from stream
        let qp_info = QPInfo {
            lid: 0,
            gid: 0,
            qp_num: 0,
            qkey: 0,
        };

        let transport = Transport::new(self.context.as_ref(), qp_info).unwrap();
        let session = Session::new(session_id, transport);
        let server_stub = ServerStub::new(session, Arc::clone(&self.handler));

        thread::spawn(move || server_stub.serve());
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
