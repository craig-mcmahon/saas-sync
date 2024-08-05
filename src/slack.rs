use serde::{Deserialize, Serialize};
use worker::{console_log, Env, Error, Response};
use trello::add_comment_to_card;
use crate::account::Account;
use crate::action::{Action, ActionService, ActionTargetSource, ActionType, ActionUpdate};
use crate::database::{get_link_from_slack_thread, Link};
use crate::trello;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum MultipleWebhookEvent {
    Challenge(Challenge),
    EventWebhook(EventWebhook),
    None,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Challenge{
    pub token: String,
    pub challenge: String,
    #[serde(rename = "type")]
    pub type_: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct Block{

}

#[derive(Serialize, Deserialize, Debug)]
pub struct Event {
    pub user: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub ts: String,
    pub text: String,
    pub team: String,
    pub thread_ts: Option<String>,
  //  pub parent_user_id: String,
  //  pub blocks: Vec<Block>,
    pub channel: String,
    pub event_ts: String,
    pub channel_type: String,
    pub bot_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EventWebhook {
    pub token: String,
    pub api_app_id: String,
    pub event: Event,
    #[serde(rename = "type")]
    pub type_: String,
    pub event_id: String,
    pub event_context: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessage {
    channel: String,
    text: String,
    thread_ts: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessageResponse {
    ok: bool,
    channel: String,
    ts: String,
    message: ChatMessageResponseMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatMessageResponseMessage {
    user: String,
    type_: String,
    ts: String,
    text: String,
}
// {
//   "message": {
//     "user": "U123456",
//     "type": "message",
//     "ts": "1715194156.015989",
//     "bot_id": "AAAA123",
//     "app_id": "AAAA123",
//     "text": "This is a test",
//     "team": "AAAA123",
//     "bot_profile": {
//       "id": "AAAA123",
//       "app_id": "AAAA123",
//       "name": "TrelloSync",
//       "icons": {
//         "image_36": "https:\/\/a.slack-edge.com\/11111\/img\/plugins\/app\/bot_36.png",
//         "image_48": "https:\/\/a.slack-edge.com\/11111\/img\/plugins\/app\/bot_48.png",
//         "image_72": "https:\/\/a.slack-edge.com\/11111\/img\/plugins\/app\/service_72.png"
//       },
//       "deleted": false,
//       "updated": 1714826485,
//       "team_id": "AAAAAAA"
//     },
//     "blocks": [
//       {
//         "type": "rich_text",
//         "block_id": "jf\/=",
//         "elements": [
//           {
//             "type": "rich_text_section",
//             "elements": [
//               {
//                 "type": "text",
//                 "text": "This is a test"
//               }
//             ]
//           }
//         ]
//       }
//     ]
//   }
// }


#[derive(Serialize, Deserialize, Debug)]
pub struct ChatPostMessageResponse{
    pub ok: bool,
    pub channel: String,
    pub ts: String,
}

// POST /api/chat.postMessage
// Content-type: application/json
// Authorization: Bearer xoxp-xxxxxxxxx-xxxx
// {"channel":"C123ABC456","text":"I hope the tour went well, Mr. Wonka.","attachments":[{"text":"Who wins the lifetime supply of chocolate?","fallback":"You could be telling the computer exactly what it can do with a lifetime supply of chocolate.","color":"#3AA3E3","attachment_type":"default","callback_id":"select_simple_1234","actions":[{"name":"winners_list","text":"Who should win?","type":"select","data_source":"users"}]}]}

const POST_MESSAGE_URL: &str = "https://slack.com/api/chat.postMessage";
const GET_USER_PROFILE_URL: &str = "https://slack.com/api/users.profile.get";

pub async fn send_action(env: &Env, action: Action) -> ChatPostMessageResponse {

    let body = ChatMessage{
        // TODO: Get channel id from config
        channel: "#test".to_string(),
        text: action.update.text,
        thread_ts: action.target.id,
    };

    let res = send_message(env, body).await;

   // console_log!("{}", res.status().as_str()
    let json: ChatPostMessageResponse = res.json().await.unwrap();
    console_log!("{:?}", &json);
    return json;
}

async fn send_message(env: &Env, body: ChatMessage) -> reqwest::Response {
    console_log!("Sending message");
   // console_log!("AT - {:?}",  env.secret("SLACK_AUTH_TOKEN".as_ref()).expect("ERR"));
    let client = reqwest::Client::new();
    let res = match client.post(POST_MESSAGE_URL)
        .header("Content-Type", "application/json; charset=utf-8")
        .header("Authorization", format!("Bearer {}", env.secret("SLACK_AUTH_TOKEN".as_ref()).unwrap().to_string()))
        .json(&body)
        .send()
        .await{
        Ok(value)=> value,
        Err(_err)=> {
            console_log!("{}", _err.to_string());
            panic!("ERR");
        },
    };

    return res;
}

pub async fn handle_webhook(webhook: EventWebhook, env: Env, _account: Account) -> worker::Result<Response> {
    console_log!("Handling webhook start");
    match &webhook.event.bot_id.as_deref() {
        None => {}, // No bot id
        _ => {
            // This is a message from a bot
            console_log!("Skipping webhook from bot account");
            return Response::ok("Skipping bot")
        },
    }

    match &webhook.event.thread_ts.as_deref() {
        None => {
            // No thread id
            console_log!("Skipping none thread message");
            return Response::ok("Skipping none thread message")
        },
        _ => {},
    }

    console_log!("Handling webhook real");

    let link = get_link_from_slack_thread(&env, &webhook.event.thread_ts.clone().unwrap()).await;


    let user = get_user(&env, &webhook.event.user).await;
    // todo: queue?
    // todo: Replace sender id with name
    console_log!("generating action");
    let action = generate_action(&webhook, link, user);

    console_log!("Running add comment");
    let  _res = add_comment_to_card(&env, action).await;

    console_log!("Woot");
    return Response::ok("Woot");
}


fn generate_action(webhook: &EventWebhook, link_result: Result<Link, Error>, user: UserProfiler) -> Action {
    let mut action: ActionType;
    let mut trello_card = None;

    let update = match &webhook.event.thread_ts{
        None => {
            action = ActionType::None;
            ActionUpdate{
                text: "".to_string(),
            }
        }
        Some(_) => {
            action = ActionType::UpdateThread;
            ActionUpdate{
                text: format!("{} posted in slack\n{}", user.display_name, webhook.event.text),
            }
        }
    };
    match link_result {
        Ok(link) => {
            trello_card = Some(link.trello_card);
        }
        Err(_) => {
            action = ActionType::None;
        }
    }
    let source = create_action_source(&webhook);
    let target = create_action_target(&webhook, trello_card);

    // No action for webhooks from apps
    match &webhook.event.bot_id {
        None => {}
        Some(_) => {
            action = ActionType::None;
        }
    }
    return Action {
        action,
        source,
        target,
        update,
    };
}


fn create_action_source(webhook: &EventWebhook) -> ActionTargetSource {
    // todo: fix url
    return ActionTargetSource {
        id: webhook.event.thread_ts.to_owned(),
        service: ActionService::Slack,
        url: "SOME URL FOR SLACK".to_string(),
    };
}

fn create_action_target(_webhook: &EventWebhook, id: Option<String>) -> ActionTargetSource {

    if let Some(value) = id.to_owned() {
        return ActionTargetSource {
            id: id.to_owned(),
            service: ActionService::Trello,
            url: format!("https://trello.com/c/{}", value),
        };
    }
    return ActionTargetSource {
        id: None,
        service: ActionService::Trello,
        url: "".to_string(),
    };
}


#[derive(Deserialize, PartialEq)]
struct UserProfiler{
    display_name: String,
}

async fn get_user(env: &Env, user_id: &str) -> UserProfiler {
    let res = send_get_user_request(env, user_id).await;
    let json: UserProfiler = res.json().await.unwrap();
    return json;
}
#[derive(Serialize)]
struct UserRequest {
    user: String
}
async fn send_get_user_request(env: &Env, user_id: &str) -> reqwest::Response {
    console_log!("getting users");
    let req = UserRequest{user: user_id.to_string()};
    let client = reqwest::Client::new();
    let res = match client.post(GET_USER_PROFILE_URL)
        .header("Content-Type", "application/x-www-form-urlencoded; charset=utf-8")
        .header("Authorization", format!("Bearer {}", env.secret("SLACK_AUTH_TOKEN".as_ref()).unwrap().to_string()))
        .form(&req)
        .send()
        .await {
        Ok(value) => value,
        Err(_err) => {
            console_log!("{}", _err.to_string());
            panic!("ERR");
        },
    };

    return res;
}

mod tests {
    use std::fs;
    use worker::Error;
    use crate::action::ActionType;
    use crate::database::Link;
    use crate::slack::{EventWebhook, UserProfiler};

    #[test]
    fn generate_action_new_thread() {
        let data = fs::read_to_string("./data/slack/new-thread.json").expect("Error reading file");

        let webhook: EventWebhook = serde_json::from_str(&data).expect("Error parsing json");
        let user = UserProfiler{display_name: "test".to_string()};

        let action = crate::slack::generate_action(&webhook, Err(Error::RustError("test".to_string())), user);
        assert_eq!(None, action.source.id);
        assert!(matches!(action.action, ActionType::None));
    }

    #[test]
    fn generate_action_unknown_thread_reply() {
        let data = fs::read_to_string("./data/slack/unknown-thread-reply.json").expect("Error reading file");

        let webhook: EventWebhook = serde_json::from_str(&data).expect("Error parsing json");
        let user = UserProfiler{display_name: "test".to_string()};
        let action = crate::slack::generate_action(&webhook, Err(Error::RustError("test".to_string())), user);
        assert_eq!(Some("1715524581.123456".to_string()), action.source.id);
        assert!(matches!(action.action, ActionType::None));
    }

    #[test]
    fn generate_action_thread_replied() {
        let data = fs::read_to_string("./data/slack/thread-replied.json").expect("Error reading file");

        let webhook: EventWebhook = serde_json::from_str(&data).expect("Error parsing json");
        let link = Link{
            slack_thread: "1715287188.123456".to_string(),
            trello_card: "ABCDEFG".to_string(),
        };
        let user = UserProfiler{display_name: "test".to_string()};

        let action = crate::slack::generate_action(&webhook, Ok(link), user);
        assert_eq!(Some("1715287188.123456".to_string()), action.source.id);
        assert!(matches!(action.action, ActionType::UpdateThread));
    }

    #[test]
    fn generate_action_thread_replied_bot() {
        let data = fs::read_to_string("./data/slack/thread-replied-bot.json").expect("Error reading file");

        let webhook: EventWebhook = serde_json::from_str(&data).expect("Error parsing json");
        let user = UserProfiler{display_name: "test".to_string()};

        let action = crate::slack::generate_action(&webhook, Err(Error::RustError("test".to_string())), user);
        assert_eq!(Some("1715287188.123456".to_string()), action.source.id);
        assert!(matches!(action.action, ActionType::None));
    }
}