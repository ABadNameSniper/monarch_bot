# an_admin_a_day
## Description
Can't agree on who should be the server owner? How about everyone?

This project will rotate the crown around all the members of a brand new server, giving you the opportunity to change each others' nicknames, make silly rules, or restructure the entire server to your every whim. Whatever you want -- as long as it's y our turn!

## How to run:
1) go to the Discord developer portal
2) Make an application, then a bot
3) go get the bot's token, export it into your environment
4) cargo run --release
5) ???
6) profit

## TODO
make it actually change the administrator every so often. maybe have a separate binary for that and just schedule systemd to run it like 4 times a day
also adding more settings for frequency of changing admin, making a list of admins, multiple admins etc.
