use std::any::{Any, TypeId};
use serde::{de, Deserialize};
use worker::{console_log, Env, Error, query};
use worker::wasm_bindgen::JsValue;
use crate::account::Account;

#[derive(Deserialize)]
pub struct Link {
   // pub id: u32,
    pub slack_thread: String,
    pub trello_card: String,
}


pub async fn get_account(env: &Env, id: &str) -> Result<Account, Error> {
    let query = "SELECT * FROM accounts WHERE id=?1";
    return get_from_db_by_id(&env, query, id).await;
}

pub async fn get_link_from_slack_thread(env: &Env, slack_thread: &str) -> Result<Link, Error> {
    let query = "SELECT * FROM links WHERE slack_thread=?1";
    return get_from_db_by_id(&env, query, slack_thread).await;
}

pub async fn get_link_from_trello_card(env: &Env, trello_card: &str) -> Result<Link, Error> {
    console_log!("Searching for trello card with id {}", trello_card);
    let query = "SELECT * FROM links WHERE trello_card=?1";
    return get_from_db_by_id(&env, query, trello_card).await;
}

pub async fn create_link(env: &Env, trello_card: &str, slack_thread: &str) -> Result<TypeId, Error> {
    console_log!("Creating link - db");
    let db = match env.d1("DB") {
        Ok(db) => db,
        Err(e) => {
            return Err(e);
        }
    };

    console_log!("Creating link - statement");
    let statement = db.prepare("insert into links values (null, ?1, ?2)");
    console_log!("Creating link - query");
    let query = statement.bind( &[JsValue::from(slack_thread), JsValue::from(trello_card)])?;

    console_log!("Creating link - result");
    let result = match query.run().await{
        Ok(result) => result,
        Err(e) => {
            console_log!("Error running query: {}", e.to_string());
            return Err(e);
        }
    };


    console_log!("Creating link - done");
    console_log!("{:?}",result.type_id());
    return Ok(result.type_id());
}

async fn get_from_db_by_id<T: de::DeserializeOwned>(env: &Env, query: &str, id: &str) -> Result<T, Error> {
    let db = match env.d1("DB") {
        Ok(db) => db,
        Err(e) => {
            return Err(e);
        }
    };

    let query = query!(
          &db,
          query,
          id,
        )?;

    let result = query.first::<T>(None).await?;

    let item = match result {
        Some(item) => item,
        None => return Err(Error::RustError("No results found".to_string())),
    };

    return Ok(item);
}