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
    transport::Transport,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, info};
use KRdmaKit::{context::Context, log::warn, services_user, QueuePairBuilder, UDriver};

#[derive(Serialize, Deserialize)]
struct SessionInfo {
    qp_info: QPInfo,
    session_id: u64,
}

pub struct Server<T, R> {
    addr: SocketAddrV4,
    ib_port: u8,
    context: Arc<Context>,
    handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
}

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("no available RDMA device")]
    NoDevice,
    #[error("no such device {0}")]
    NoSuchDevice(String),
    #[error("rdma error, {0}")]
    Rdma(String),
    #[error("error binding port, {0}")]
    TcpBind(String),
}

impl<T, R> Server<T, R>
where
    T: DeserializeOwned + 'static,
    R: Serialize + 'static + Clone,
{
    pub fn new(
        dev: &str,
        ib_port: u8,
        addr: SocketAddrV4,
        handler: Arc<dyn RpcHandler<Args = T, Resp = R>>,
    ) -> Result<Server<T, R>, ServerError> {
        // create context
        let context = {
            let udriver = UDriver::create().ok_or(ServerError::NoDevice)?;
            let device = udriver
                .devices()
                .iter()
                .find(|d| d.name() == dev)
                .ok_or_else(|| ServerError::NoSuchDevice(dev.to_string()))?;
            device
                .open_context()
                .map_err(|e| ServerError::Rdma(e.to_string()))?
        };
        Ok(Self {
            addr,
            ib_port,
            context,
            handler,
        })
    }

    pub fn serve(self) -> Result<(), ServerError> {
        info!("server start listening on {}", self.addr);
        let listener =
            TcpListener::bind(self.addr).map_err(|e| ServerError::TcpBind(e.to_string()))?;

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => self.handle_client(stream),
                Err(e) => {
                    error!("accepting tcp listener failed, {e}")
                }
            }
        }

        Ok(())
    }

    pub fn handle_client(&self, mut stream: TcpStream) {
        let context = Arc::clone(&self.context);
        let ib_port = self.ib_port;
        let handler = Arc::clone(&self.handler);
        thread::spawn(move || {
            // create a new session
            let session_id = rand::random();

            // receive QPInfo from stream
            let mut buf = [0; 1024];
            let size = match stream.read(&mut buf) {
                Ok(size) => size,
                Err(err) => {
                    warn!("failed to read qp info from client, {err}");
                    warn!("closing session {session_id}");
                    return;
                }
            };
            let client_qp_info: QPInfo = match bincode::deserialize(&buf[0..size]) {
                Ok(qp_info) => qp_info,
                Err(err) => {
                    warn!("failed to deserialize client qp info, {err}");
                    warn!("closing session {session_id}");
                    return;
                }
            };
            info!("client qp info: {client_qp_info}");

            // create transport and self qp_info
            let transport = match Transport::new(context, client_qp_info, ib_port) {
                Ok(transport) => transport,
                Err(e) => {
                    warn!("failed to create transport for client, {e}");
                    warn!("closing session {session_id}");
                    return;
                }
            };
            let qp_info = transport.qp_info();

            // send back self info
            info!("server qp info: {qp_info}, session_id: {session_id}");
            let session_info = SessionInfo {
                qp_info,
                session_id,
            };
            let session_info = bincode::serialize(&session_info).unwrap();
            if let Err(err) = stream.write_all(&session_info) {
                warn!("failed to send session info to the client, {err}");
            }

            // start serving
            let session = Session::new(session_id, transport);
            let server_stub = ServerStub::new(session, handler);
            info!("session {session_id} start serving");
            server_stub.serve()
        });
    }
}

pub struct Client<T, R> {
    client_stub: ClientStub,
    #[allow(unused)] // Reserve for future usage
    context: Arc<Context>,
    phantom_t: PhantomData<T>,
    phantom_r: PhantomData<R>,
}

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("no available RDMA device")]
    NoDevice,
    #[error("no such device {0}")]
    NoSuchDevice(String),
    #[error("rdma error, {0}")]
    Rdma(String),
    #[error("connect failed, {0}")]
    Connect(String),
    #[error("internal error, {0}")]
    Internal(String),
}

impl<T, R> Client<T, R>
where
    T: Serialize + 'static,
    R: DeserializeOwned + 'static,
{
    pub fn new(dev: &str, addr: SocketAddrV4, ib_port: u8) -> Result<Client<T, R>, ClientError> {
        // create context
        let context = {
            let udriver = UDriver::create().ok_or(ClientError::NoDevice)?;
            let device = udriver
                .devices()
                .iter()
                .find(|d| d.name() == dev)
                .ok_or_else(|| ClientError::NoSuchDevice(dev.to_string()))?;
            device
                .open_context()
                .map_err(|e| ClientError::Rdma(e.to_string()))?
        };

        // create qp
        let qp = QueuePairBuilder::new(&context)
            .build_ud()
            .map_err(|err| ClientError::Rdma(format!("failed to build ud, {err}")))?
            .bring_up_ud()
            .map_err(|err| ClientError::Rdma(format!("failed to bring up ud, {err}")))?;

        // send self qp info
        let client_qp_info = QPInfo {
            lid: qp.lid().unwrap(),
            gid: services_user::ibv_gid_wrapper::from(qp.gid().unwrap()),
            qp_num: qp.qp_num(),
            qkey: qp.qkey(),
        };
        info!("client send self qp info: {client_qp_info}");
        let mut stream =
            TcpStream::connect(addr).map_err(|err| ClientError::Connect(err.to_string()))?;
        let data = bincode::serialize(&client_qp_info).unwrap();
        stream.write_all(&data).map_err(|err| {
            ClientError::Connect(format!("failed to send self qp_info to the server, {err}"))
        })?;

        // receive session info
        let mut buf = [0; 1024];
        let size = stream.read(&mut buf).map_err(|err| {
            ClientError::Connect(format!("failed to recv session info from server, {err}"))
        })?;
        let SessionInfo {
            qp_info,
            session_id,
        } = bincode::deserialize(&buf[0..size]).map_err(|err| {
            ClientError::Connect(format!("failed to deserialize session info, {err}"))
        })?; // TODO: handle error
        info!("client recv server qp info: {qp_info}, session id: {session_id}");

        // create client stub
        let tranport = Transport::new_with_qp(qp, Arc::clone(&context), qp_info, ib_port).unwrap();
        let session = Session::new(session_id, tranport);
        let client_stub = ClientStub::new(session);

        Ok(Self {
            client_stub,
            context,
            phantom_t: PhantomData,
            phantom_r: PhantomData,
        })
    }

    pub fn send(&mut self, args: T) -> Result<R, ClientError> {
        Ok(self.client_stub.sync_call(args)?)
    }
}

impl From<rdma_rpc_core::error::Error> for ClientError {
    fn from(err: rdma_rpc_core::error::Error) -> Self {
        ClientError::Internal(err.to_string())
    }
}
