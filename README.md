# Lupin

A simple cli tool to backup trello boards with attachments.

## Install

```sh
cargo install --git https://github.com/hov1417/lupin.git
```

## Config

```yaml
# ~/.config/lupin.yml
auth_cookie: "..." # trello cookie, copy it from browser open trello.com login, then
               # open devtools -> network -> open any request -> Request headers -> cookie   
board_ids: # list of board ids to backup (you can get it from url https://trello.com/b/{board_id}/{board_name} )
  - "id1"
  - "id2"
out_path: "~/trello-backup/" # path where lupin will make a trello backups

```

## Usage

Export all backups

```sh
lupin get
```

