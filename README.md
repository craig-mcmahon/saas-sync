# SAAS Sync

This is a WIP project created to help with learning rust, and to solve a problem with data syncing between services
Trello and Slack. It is designed to be deployed as a CloudFlare worker.

## Current state

Once setup updates from Trello will create a thread in a Slack channel and store the thread id, subsequent updates to
the same card will reply in the thead. A reply to the thread from within Slack will create a new comment on the card.


## Setup

Required dependencies include
- rust
- node/npm/npx
- more?

### Install database locally

```
npx wrangler d1 execute my_db --local --file schema.sql
```


