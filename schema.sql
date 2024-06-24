DROP TABLE IF EXISTS links;
CREATE TABLE IF NOT EXISTS links (
   id integer PRIMARY KEY AUTOINCREMENT,
   slack_thread nvarchar(100),
   trello_card nvarchar(100)
    );
CREATE UNIQUE INDEX idx_slack ON links (slack_thread);
CREATE UNIQUE INDEX idx_trello ON links (trello_card);

DROP TABLE IF EXISTS accounts;
CREATE TABLE IF NOT EXISTS accounts (
   id uuid_str(4) PRIMARY KEY,
   name nvarchar(256)
);

insert into accounts values ('92cfdda8-bb81-480c-b3ca-092d3366b244', 'Test Account');
