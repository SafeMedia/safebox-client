use crate::{Deserialize, Error, Serialize};

pub(crate) const USER_SESSION_KEY: &str = "user";

#[derive(Serialize)]
pub(crate) struct SimpleAccountUser {
    pub(crate) username: String,
    pub(crate) address: String,
}
