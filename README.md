# monarch_bot

![Profile Picture](monarch_bot_completed_pfp.png)

[Image by Alien Squid Boi](https://www.instagram.com/alien.squid.boi/)

## Description
Can't agree on who should be the server owner? How about everyone?

This project will rotate the crown around all the members of a brand new server, giving you the opportunity to change each others' nicknames, make silly rules, or restructure the entire server to your every whim. Whatever you want -- as long as it's your turn!

## How to build:
1) `cargo run --bin setup`
2) `cargo run --bin switch_monarch`
3) ???
4) profit

## How to run:
1) go to the Discord developer portal
2) Make an application, then a bot
3) go get the bot's token, export it into your environment
4) run setup, and then switch_monarch as needed (i have systemd run it once a day)

## TODO
* fix setting server icon to avatarless users. make sure it supports animated avatars too
  * test this
* check to see if everyone on list is still in the server before changing monarch
