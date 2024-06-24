use serde::Deserialize;
use url::form_urlencoded::byte_serialize;
use worker::{console_log, Env, Error, Response};
use crate::account::Account;
use crate::action::{Action, ActionService, ActionTargetSource, ActionType, ActionUpdate};
use crate::database::{create_link, get_link_from_trello_card, Link};
use crate::slack::send_action;

#[derive(Deserialize, Debug)]
pub struct TrelloWebhook {
    pub model: TrelloWebhookModel,
    pub action: TrelloWebhookAction,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookModel {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub url: String,
    pub short_url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookAction {
    pub id: String,
    pub id_member_creator: String,
    data: TrelloWebhookActionData,
    #[serde(rename = "type")]
    pub date: String,
    pub display: TrelloWebhookActionDisplay,
    pub app_creator: Option<TrelloWebhookActionAppCreator>
}


#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionAppCreator {
    pub id: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionData {
    pub id: Option<String>,
    pub text: Option<String>,
    pub card: TrelloWebhookActionCard,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionCard {
    pub id: String,
    pub name: String,
    pub id_short: u32,
    pub short_link: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionDisplay {
    pub translation_key: ActionDisplayTranslationKey,
    pub entities: TrelloWebhookActionDisplayEntities,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionDisplayEntities {
    pub card: TrelloWebhookActionDisplayEntitiesCard,
    pub member_creator: TrelloWebhookActionDisplayMemberCreator,
    pub list_before: Option<TrelloWebhookActionDisplayListBeforeAfter>,
    pub list_after: Option<TrelloWebhookActionDisplayListBeforeAfter>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionDisplayEntitiesCard {
    #[serde(rename = "type")]
    pub type_: String,
    pub closed: Option<bool>,
    pub desc: Option<String>,
    pub id: String,
    pub short_link: String,
    pub text: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionDisplayMemberCreator {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub username: String,
    pub text: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all(serialize = "snake_case", deserialize = "camelCase"))]
pub struct TrelloWebhookActionDisplayListBeforeAfter {
    #[serde(rename = "type")]
    pub type_: String,
    pub id: String,
    pub text: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all(serialize = "PascalCase", deserialize = "snake_case"))]
pub enum ActionDisplayTranslationKey {
    ActionCreateCard,
    ActionArchivedCard,
    ActionCommentOnCard,
    ActionChangedDescriptionOfCard,
    ActionMoveCardFromListToList,
    ActionRenamedCard,
    ActionMovedCardLower,
    #[serde(untagged)]
    Unknown(String),
}

pub async fn handle_webhook(env: Env, webhook: TrelloWebhook, _account: Account) -> worker::Result<Response> {
    let link = get_link_from_trello_card(&env, &webhook.action.display.entities.card.id).await;
    let action = generate_action(&webhook, link);
    console_log!("Generated action -> {}", &action.update.text);

    match action.action {
        ActionType::NewThread => {
            console_log!("New thread");
            let response = send_action(&env, action).await;
            create_link(&env, &webhook.action.display.entities.card.id, &response.ts).await.expect("Error inserting row");
        }
        ActionType::UpdateThread => {
            console_log!("Existing thread");
            send_action(&env, action).await;
        }
        ActionType::None => {}
    }


    return Response::ok("Success");
}

fn generate_action(webhook: &TrelloWebhook, link_result: Result<Link, Error>) -> Action {
    let mut action = ActionType::UpdateThread;
    let mut slack_id = None;
    match link_result {
        Ok(link) => {
            slack_id = Some(link.slack_thread);
        }
        Err(_) => {
            action = ActionType::NewThread;
        }
    }

    let source = create_action_source(&webhook);
    let target = create_action_target(&webhook, slack_id);

    let update = match &webhook.action.display.translation_key {
        ActionDisplayTranslationKey::ActionCreateCard => handle_card_created(&webhook),
        ActionDisplayTranslationKey::ActionArchivedCard => handle_archived_card(&webhook),
        ActionDisplayTranslationKey::ActionRenamedCard => handle_card_renamed(&webhook),
        ActionDisplayTranslationKey::ActionChangedDescriptionOfCard => handle_description_updated(&webhook),
        ActionDisplayTranslationKey::ActionCommentOnCard => handle_comment_added(&webhook),
        ActionDisplayTranslationKey::ActionMoveCardFromListToList => handle_card_moved(&webhook),
        ActionDisplayTranslationKey::Unknown(value) => {
            action = ActionType::None;
            ActionUpdate{
                text: format!("Unknown key {:?}", value)
            }
        }
        _ => {
            action = ActionType::None;
            ActionUpdate{
                text: format!("Unknown key {:?}", &webhook.action.display.translation_key)
            }
        }
    };

    // No action for webhooks from apps
    match &webhook.action.app_creator {
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

fn handle_card_created(webhook: &TrelloWebhook) -> ActionUpdate {
    return ActionUpdate {
        text: format!("This card {} has been created by {}", webhook.action.display.entities.card.text, webhook.action.display.entities.member_creator.text),
    };
}
fn handle_archived_card(webhook: &TrelloWebhook) -> ActionUpdate {
    return ActionUpdate {
        text: format!("This card has been archived by {}", webhook.action.display.entities.member_creator.text),
    };
}

fn handle_card_renamed(webhook: &TrelloWebhook) -> ActionUpdate {
    return ActionUpdate {
        text: format!("This card has been renamed to {} by {}",
                      webhook.action.display.entities.card.text,
                      webhook.action.display.entities.member_creator.text),
    };
}

fn handle_card_moved(webhook: &TrelloWebhook) -> ActionUpdate {
    return ActionUpdate {
        text: format!("This card has been moved from list {} to list {} by {}",
                      webhook.action.display.entities.list_before.clone().unwrap().text,
                      webhook.action.display.entities.list_after.clone().unwrap().text,
                      webhook.action.display.entities.member_creator.text),
    };
}

fn handle_description_updated(webhook: &TrelloWebhook) -> ActionUpdate {
    return ActionUpdate {
        text: format!("This card description has been updated to {} by {}",
                      webhook.action.display.entities.card.desc.clone().unwrap(),
                      webhook.action.display.entities.member_creator.text),
    };
}

fn handle_comment_added(webhook: &TrelloWebhook) -> ActionUpdate {
    return ActionUpdate {
        text: format!("Comment added by {}\n{}",
                      webhook.action.display.entities.member_creator.text,
                      webhook.action.data.text.clone().unwrap(),
        ),
    };
}


fn create_action_source(webhook: &TrelloWebhook) -> ActionTargetSource {
    return ActionTargetSource {
        id: Option::from(String::from(&webhook.action.data.card.id)),
        service: ActionService::Trello,
        url: format!("https://trello.com/c/{}", webhook.action.data.card.short_link),
    };
}

fn create_action_target(_webhook: &TrelloWebhook, id: Option<String>) -> ActionTargetSource {
    // todo: fix url
    return ActionTargetSource {
        id,
        service: ActionService::Slack,
        url: "SOME URL FOR SLACK".to_string(),
    };
}



pub async fn add_comment_to_card(env: &Env, action: Action){
    if action.action == ActionType::None {
        return;
    }

    let api_key = env.secret("TRELLO_API_KEY".as_ref()).unwrap().to_string();
    let api_token =  env.secret("TRELLO_API_TOKEN".as_ref()).unwrap().to_string();

    let card_id = action.target.id.unwrap().to_string();
    let text: String = byte_serialize( action.update.text.as_bytes()).collect();

    let url = format!("https://api.trello.com/1/cards/{card_id}/actions/comments?text={text}&key={api_key}&token={api_token}");

    let client = reqwest::Client::new();
    let _res = match client.post(url)
        .header("Content-Type", "application/json")
        .send()
        .await{
        Ok(value)=> value,
        Err(_err)=> {
            console_log!("{}", _err.to_string());
            panic!("ERR");
        },
    };

    //return res;
}


mod tests {
    use std::fs;
    use worker::{Error};
    use crate::trello::{generate_action, TrelloWebhook};

    #[test]
    fn generate_action_card_archived() {
        let data = fs::read_to_string("./data/trello/card-archived.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::NewThread));
        assert!(action.update.text.contains("TEST UPDATED NAME"));
        assert!(action.update.text.contains("archived"));
    }

    #[test]
    fn generate_action_card_title_changed() {
        let data = fs::read_to_string("./data/trello/card-title-changed.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::NewThread));
        assert!(action.update.text.contains("TEST UPDATED NAME"));
        assert!(action.update.text.contains("changed title"));
        assert!(action.update.text.contains("renamed"));
    }

    #[test]
    fn generate_action_card_moved() {
        let data = fs::read_to_string("./data/trello/card-moved.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::NewThread));
        assert!(action.update.text.contains("TEST UPDATED NAME"));
        assert!(action.update.text.contains("moved"));
        assert!(action.update.text.contains("from list Doing"));
        assert!(action.update.text.contains("to list Done"));
    }

    #[test]
    fn generate_action_card_description_updated() {
        let data = fs::read_to_string("./data/trello/card-description-edited.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::NewThread));
        assert!(action.update.text.contains("TEST UPDATED NAME"));
        assert!(action.update.text.contains("description"));
        assert!(action.update.text.contains("This is the description being edited"));
    }

    #[test]
    fn generate_action_card_comment_added() {
        let data = fs::read_to_string("./data/trello/card-comment-added.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::NewThread));
        assert!(action.update.text.contains("TEST UPDATED NAME"));
        assert!(action.update.text.contains("Comment added"));
        assert!(action.update.text.contains("This is a new comment"));
    }


    #[test]
    fn generate_action_card_copied() {
        // Not handled at the moment, but make sure it doesn't break
        let data = fs::read_to_string("./data/trello/card-copied.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::None));
        assert!(action.update.text.contains("action_copy_card"));
        assert!(action.update.text.contains("Unknown"));
    }


    #[test]
    fn generate_action_card_comment_added_from_api() {
        let data = fs::read_to_string("./data/trello/card-comment-added-from-api.json").expect("Error reading file");

        let webhook: TrelloWebhook = serde_json::from_str(&data).expect("Error parsing json");

        let action = generate_action(&webhook, Err(Error::RustError("test".to_string())));
        assert_eq!("abc64ds5ad45s6161d", action.source.id.unwrap());
        assert!(matches!(action.action, crate::action::ActionType::None));
    }


}