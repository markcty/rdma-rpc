use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, session::Session};

pub struct ClientStub {
    session: Session,
}

impl ClientStub {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    pub fn sync_call<T: Serialize, R: DeserializeOwned>(&mut self, args: T) -> Result<R, Error> {
        // remote call
        self.session.send(args)?;
        self.session.recv()
    }
}
