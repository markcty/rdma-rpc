use serde::{de::DeserializeOwned, Serialize};
use KRdmaKit::{context::Context, log::error};

use crate::{
    error::Error,
    messages::{Packet, QPInfo},
};

pub struct Transport {}

impl Transport {
    /// Connect to the server and
    pub fn new(context: &Context, qp_info: QPInfo) -> Result<Self, Error> {
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

    pub(crate) fn recv<R: DeserializeOwned>(&self) -> Result<Packet<R>, Error> {
        // serialize arg
        // TODO: deserialize directly from memory region
        let buffer = [0; 233];
        let req = bincode::deserialize(&buffer).map_err(|err| {
            error!("failed to serialize rpc args, {err}");
            Error::EncodeArgs
        })?;

        // post send

        Ok(req)
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
