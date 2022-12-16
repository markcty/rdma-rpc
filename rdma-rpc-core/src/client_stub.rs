use serde::{de::DeserializeOwned, Serialize};

use crate::{error::Error, session::Session};

pub struct ClientStub {
    session: Session,
}

impl ClientStub {
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    pub fn sync_call<T: Serialize + Clone, R: DeserializeOwned + Clone>(
        &mut self,
        args: T,
    ) -> Result<R, Error> {
        // remote call
        self.session.send(args)?;
        // self.session.send_u8(args)?;
        self.session.recv()
    }
}
