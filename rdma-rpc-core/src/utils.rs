use alloc::{sync::Arc, vec::Vec};
use serde::de::DeserializeOwned;
use KRdmaKit::QueuePair;

use crate::{error::Error, messages::Packet};

pub(crate) fn poll_packets<T: DeserializeOwned>(qp: &QueuePair) -> Result<Vec<Packet<T>>, Error> {
    Ok(Vec::new())
}
