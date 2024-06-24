use serde::Deserialize;
use worker::{Env, Error};

#[derive(Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
}

pub async fn get_account(env: &Env, id: &str) -> Result<Account, Error> {
    let id = match env.secret("ACCOUNT_ID".as_ref()) {
        Ok(val) => val.to_string(),
        Err(_) => return crate::database::get_account(&env, id).await
    };

    return Ok(Account{
        id,
        name: "test".to_string(),
    });
}
