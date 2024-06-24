mod trello;
mod action;
mod slack;
mod database;
mod account;

use serde::{Deserialize, Serialize};
use worker::*;
use crate::account::{Account, get_account};
use crate::slack::{MultipleWebhookEvent};
use crate::trello::{TrelloWebhook};

#[derive(Debug, Deserialize, Serialize)]
struct GenericResponse {
    status: u16,
    message: String,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_log!(
        "{} {}, located at: {:?}, within: {}",
        req.method().to_string(),
        req.path(),
        req.cf().unwrap().coordinates().unwrap_or_default(),
        req.cf().unwrap().region().unwrap_or("unknown region".into())
    );


    Router::new()
        .get_async("/", handle_default)
        .post_async("/trello-webhook/:id", trello_webhook_hit)
        .post_async("/slack-webhook/:id", slack_webhook)
        .head_async("/trello-webhook/:id", trello_webhook_setup)
        .run(req, env)
        .await
}

async fn handle_default(_: Request, _ctx: RouteContext<()>) -> Result<Response> {
    return Response::ok("Default");
}

async fn trello_webhook_setup(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    if let Some(id) = ctx.param("id") {
        let account = get_account(&ctx.env, id).await;
        return match account {
            Ok(_) => Response::ok("Success"),
            _ => Response::error("Not found", 404),
        };
    }

    return  Response::error("Not found", 404);
}

async fn trello_webhook_hit(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let account = match get_account_from_request(&ctx).await {
        Ok(account) => account,
        _ => return Response::error("Not found", 404),
    };

    let webhook:TrelloWebhook = match req.json().await{
        Ok(value)=> value,
        Err(err)=>return Response::error(err.to_string(),400),
    };

    return trello::handle_webhook(ctx.env, webhook, account).await;
}


async fn slack_webhook(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let account = match get_account_from_request(&ctx).await {
        Ok(account) => account,
        _ => return Response::error("Not found", 404),
    };

    let webhook :MultipleWebhookEvent = match req.json().await{
        Ok(value) => value,
        Err(err) => return Response::error(err.to_string(),400),
    };
    return match webhook {
        MultipleWebhookEvent::Challenge(challenge) => Response::ok(challenge.challenge),
        MultipleWebhookEvent::EventWebhook(event) => slack::handle_webhook(event, ctx.env, account).await,
        _ => Response::error("Bad request", 400),
    };
}

async fn get_account_from_request(ctx: &RouteContext<()>) -> Result<Account>{
    return if let Some(id) = ctx.param("id") {
        match get_account(&ctx.env, id).await {
            Ok(account) => Ok(account),
            _ => Err(Error::RustError("Not Found".to_string())),
        }
    } else {
        Err(Error::RustError("Not Found".to_string()))
    }
}