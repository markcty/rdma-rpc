use serde::Serialize;
use KRdmaKit::log::error;

use crate::{
    error::Error,
    messages::{Packet, QPInfo},
};

pub struct Transport {}

impl Transport {
    /// Connect to the server and
    pub fn new(qp_info: QPInfo) -> Result<Self, Error> {
        // create a qp and the remote endpoint
        Ok(Self {})
    }

    pub(crate) fn send<T: Serialize>(&self, session_id: u64, data: T) -> Result<(), Error> {
        let packet = Packet::new(session_id, data);

        // serialize arg
        // TODO: serialize directly into memory region
        let req = bincode::serialize(&packet).map_err(|err| {
            error!("failed to serialize rpc args, {err}");
            Error::EncodeArgs
        })?;

        // post send

        Ok(())
    }

    /// Get self qp info
    pub fn qp_info(&self) -> QPInfo {
        QPInfo {
            lid: 0,
            gid: 0,
            qp_num: 0,
            qkey: 0,
        }
    }
}
