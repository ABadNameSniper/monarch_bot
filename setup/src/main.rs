use std::{fs, fs::*};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_model::{
    gateway::{
        payload::outgoing::UpdatePresence,
        presence::{
            Activity, ActivityType, MinimalActivity, Status
        }
    }, 
    id::Id,
    guild::Permissions
};
use twilight_http::Client;
use std::io;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let mut new_guild = true;

    let (delete_guild, server_id): (bool, u64) = match File::options()
        .read(true)
        .write(true)
        .create_new(true)
        .open("guild_id.txt") 
    {
        Ok(_/*mut file*/) => {
            // let mut buf = String::new();
            // file.read_to_string(&mut buf)?;
            //(false, buf.parse::<u64>().unwrap())
            (false, 0) //chatchpt said default values are ok
        },
        Err(_) => {
            let server_id: u64 = fs::read_to_string("guild_id.txt")?
                .parse()
                .expect("Couldn't parse file. Do you have a valid guild id?");

            println!(
                "Looks like you already have a server. Press enter to continue or \"RESTART WITH NEW SERVER\" to restart with a new server. \"DELETE CURRENT SERVER\" will destroy the current server without creating a new one."
            );

            let mut response = String::new();

            io::stdin()
                .read_line(&mut response)
                .expect("Something went wrong reading input");

            println!("Response: {response}");

            let delete_guild = match response.trim() {
                "RESTART WITH NEW SERVER" => {
                    true
                },
                "DELETE CURRENT SERVER" => {
                    new_guild = false;
                    true
                },
                _ => {
                    panic!("Invalid response. Program exiting.");
                }
            };

            (delete_guild, server_id)
        }
    };

    let token = match fs::read_to_string("bot_token.txt") {
        Ok(result) => result,
        Err(_) => {
            println!("You'll need to save your bot token to run the program automatically. Paste it here and it will get saved to bot_token.txt");

            let mut response = String::new();
            
            io::stdin()
                .read_line(&mut response)
                .expect("Something went wrong reading input");

            response = response.trim().to_string();

            fs::write("bot_token.txt", &response)?;

            response
        }
    };

    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let mut shard = Shard::new(
        ShardId::ONE,
        token.to_owned(),
        Intents::GUILD_MEMBERS | Intents::GUILDS,
    );

    let client = Client::new(token.to_owned());

    loop { let event = match shard.next_event().await {
        Ok(event) => match &event {
            Event::Ready(_) => {

                println!("Ready for action!");

                let minimal_activity = MinimalActivity {
                    kind: ActivityType::Playing,
                    name: if new_guild {"Setting up a brand new server!".to_owned()} else {"Deleting old server... Goodbye.".to_owned()},
                    url: None,
                };
                let command = UpdatePresence::new(
                    Vec::from([Activity::from(minimal_activity)]),
                    false,
                    Some(1),
                    Status::Online,
                )?;

                shard.command(&command).await?;
                println!("Status set!");

                if delete_guild {
                    let discord_response = client
                        .delete_guild(Id::new_checked(server_id).unwrap())
                        .await?;

                    println!("Response from discord: {discord_response:?}");
                    println!("Destroyed old server!");

                    let files = vec!["guild_id.txt", "monarch_user_id.txt", "monarch_role_id.txt", "remaining_monarchs.json"];

                    for file in files {
                        match fs::remove_file(file) {
                            Ok(()) => println!("File {} deleted successfully", file),
                            Err(error) => println!("Could not delete file {}: {}", file, error),
                        }
                    }

                    println!("Deleted records of old server");

                    if !new_guild {
                        println!("No new guild... exiting!");
                        break;
                    }
                }

                let new_guild = client
                    .create_guild(String::from("Brand New Server"))
                    .expect("Invalid Name!")
                    .await?
                    .model()
                    .await?;
                    
                let new_system_channel_id = new_guild.system_channel_id.expect("Couldn't get the system channel ID");
                println!("Guild created!");

                //save the server id
                fs::write("guild_id.txt", new_guild.id.get().to_string())?;
                println!("Guild ID saved");

                // This doesn't return the code
                let new_invite = client.create_invite(new_system_channel_id).max_age(0)?.await?;
                // Get the code from here instead
                let new_channel_id = new_invite.model().await?.channel.expect("oops no channel").id;
                let channel_invites = client.channel_invites(new_channel_id).await?;
                let new_invite_code = &channel_invites.model().await?[0].code;

                println!("Invite code: discord.gg/{new_invite_code}");

                let monarch_permission = Permissions::ADMINISTRATOR;

                let monarch_role = client.create_role(new_guild.id)
                    .name("The Monarch")
                    .permissions(monarch_permission)
                    .await?;

                let monarch_role_id = monarch_role.model().await?.id;

                println!("Monarch role created. Waiting for first person to join");

                fs::write("monarch_role_id.txt", monarch_role_id.get().to_string())?;
                println!("Monarch role saved to file");

                fs::write("remaining_monarchs.json", "")?;

                break
                
            }
            _ => {}
        },
        Err(source) => {
            tracing::warn!(?source, "error receiving event");

            if source.is_fatal() {
                break;
            }

            continue;
        }
    }; tracing::debug!(?event, "event"); }

    Ok(())
}