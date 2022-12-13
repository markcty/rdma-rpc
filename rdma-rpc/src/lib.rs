extern crate alloc;

use std::{
    io::{Read, Write},
    marker::PhantomData,
    net::{SocketAddrV4, TcpListener, TcpStream},
    thread,
};

use alloc::sync::Arc;
use rdma_rpc_core::{
    client_stub::ClientStub,
    messages::QPInfo,
    server_stub::{RpcHandler, ServerStub},
    session::Session,
    transport::{Transport, BUF_SIZE},
};
use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{context::Context, services_user, QueuePairBuilder, UDriver};

pub struct Server<T, R> {
    addr: SocketAddrV4,
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
        // create context
        let context = {
            let udriver = UDriver::create().expect("no device");
            let device = if let Some(device) = udriver.devices().iter().find(|d| d.name() == dev) {
                device
            } else {
                return Err(Error);
            };
            device.open_context().map_err(|_| Error)?
        };
        Ok(Self {
            addr,
            ib_port,
            context,
            handler,
        })
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

    pub fn handle_client(&self, mut stream: TcpStream) {
        // create a new session
        let session_id = rand::random();

        // receive QPInfo from stream
        let mut buf = [0; BUF_SIZE as usize];
        let size = stream.read(&mut buf).expect("failed to read client info");
        let client_qp_info: QPInfo =
            bincode::deserialize(&buf[0..size]).expect("failed to deserialize qp info"); // TODO: handle error

        // create qp
        // The reason why we create the qp so early (outside the transport) is that we need to use tcp to send back server's qp info later,
        // and we have to create the transport in the new thread to escape Send ah (the AddressHandler didn't impl Send)
        let qp = QueuePairBuilder::new(&self.context)
            .build_ud()
            .expect("failed to build UD QP")
            .bring_up_ud()
            .expect("failed to bring up UD QP");

        // send back server_qp_info
        let server_qp_info = QPInfo {
            lid: qp.lid().unwrap(),
            gid: services_user::ibv_gid_wrapper::from(qp.gid().unwrap()),
            qp_num: qp.qp_num(),
            qkey: qp.qkey(),
            session_id: Some(session_id),
        };
        let data = bincode::serialize(&server_qp_info).unwrap();
        let res = stream.write_all(&data);
        if res.is_err() {
            // TODO: handle error
        }

        let context = Arc::clone(&self.context);
        let handler = Arc::clone(&self.handler);
        let port = self.ib_port;
        thread::spawn(move || {
            let transport = Transport::new_with_qp(qp, context, client_qp_info, port).unwrap();
            let session = Session::new(session_id, transport);
            let server_stub = ServerStub::new(session, handler);
            server_stub.serve()
        });
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
    pub fn new(dev: &str, addr: SocketAddrV4, ib_port: u8) -> Result<Client<T, R>, Error> {
        // create context and qp
        let context = {
            let udriver = UDriver::create().expect("no device");
            let device = if let Some(device) = udriver.devices().iter().find(|d| d.name() == dev) {
                device
            } else {
                return Err(Error);
            };
            device.open_context().map_err(|_| Error)?
        };
        let qp = QueuePairBuilder::new(&context)
            .build_ud()
            .expect("failed to build UD QP")
            .bring_up_ud()
            .expect("failed to bring up UD QP");

        // send and receive initial information
        let client_qp_info = QPInfo {
            lid: qp.lid().unwrap(),
            gid: services_user::ibv_gid_wrapper::from(qp.gid().unwrap()),
            qp_num: qp.qp_num(),
            qkey: qp.qkey(),
            session_id: None,
        };
        let mut stream = TcpStream::connect(addr).expect("connect failed");
        let data = bincode::serialize(&client_qp_info).unwrap();
        let res = stream.write_all(&data);
        if res.is_err() {
            // TODO: handle error
        }
        let mut buf = [0; BUF_SIZE as usize];
        let size = stream.read(&mut buf).expect("failed to read client info");
        let server_qp_info: QPInfo =
            bincode::deserialize(&buf[0..size]).expect("failed to deserialize qp info"); // TODO: handle error

        let session_id = server_qp_info.session_id.unwrap();
        let client_stub =
            ClientStub::new(qp, context, server_qp_info, session_id, ib_port).map_err(|_| Error)?;

        Ok(Self {
            client_stub,
            phantom_t: PhantomData,
            phantom_r: PhantomData,
        })
    }

    pub fn send(&self, args: T) -> Result<R, Error> {
        Ok(self.client_stub.sync_call(args).unwrap()) // TODO: handle error
    }
}
