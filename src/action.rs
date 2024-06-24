use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Action {
    pub action: ActionType,
    pub target: ActionTargetSource,
    pub source: ActionTargetSource,
    pub update: ActionUpdate,
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub enum ActionType {
    NewThread,
    UpdateThread,
    None,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ActionTargetSource {
    pub id: Option<String>,
    pub url: String,
    pub service: ActionService,
}

#[derive(Deserialize, Clone, Debug)]
pub enum ActionService {
    Slack,
    Trello,
}

#[derive(Deserialize, Clone, Debug)]
pub struct ActionUpdate {
    pub text: String,
}