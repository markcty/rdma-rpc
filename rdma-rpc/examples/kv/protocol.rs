use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Args {
    Get(i32),
    Put(i32, i32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Resp {
    Get(Option<i32>),
    Put,
}
