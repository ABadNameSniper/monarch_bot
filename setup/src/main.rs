use std::{fs, fs::*};
use setup::Configuration;
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_model::{
    gateway::{
        payload::outgoing::UpdatePresence,
        presence::{
            Activity, ActivityType, MinimalActivity, Status
        }
    }, 
    id::{Id, marker::{UserMarker, GuildMarker}},
    guild::Permissions
};
use twilight_http::Client;
use std::io;
use serde_json;

// I don't know how to separate out creating events and managing them!
// TODO: create a default permissions file. It literally just needs to be a 64 bitfield/int/whatever.
// use these as backup (maybe remove @everyone, and managing events/emojis)
const DEFAULT_PERMISSIONS: Permissions = Permissions::from_bits_truncate(
    Permissions::CREATE_INVITE.bits()       | Permissions::ADD_REACTIONS.bits()             | 
    Permissions::STREAM.bits()                  | Permissions::VIEW_CHANNEL.bits()              | 
    Permissions::SEND_MESSAGES.bits()           | Permissions::EMBED_LINKS.bits()               | 
    Permissions::ATTACH_FILES.bits()            | Permissions::READ_MESSAGE_HISTORY.bits()      | 
    Permissions::MENTION_EVERYONE.bits()        | Permissions::USE_EXTERNAL_EMOJIS.bits()       | 
    Permissions::CONNECT.bits()                 | Permissions::SPEAK.bits()                     |
    Permissions::USE_VAD.bits()                 | Permissions::CHANGE_NICKNAME.bits()           | 
    Permissions::MANAGE_GUILD_EXPRESSIONS.bits()| Permissions::USE_SLASH_COMMANDS.bits()        |
    Permissions::REQUEST_TO_SPEAK.bits()        | Permissions::MANAGE_EVENTS.bits()             |
    Permissions::CREATE_PUBLIC_THREADS.bits()   | Permissions::CREATE_PRIVATE_THREADS.bits()    | 
    Permissions::USE_EXTERNAL_STICKERS.bits()   | Permissions::SEND_MESSAGES_IN_THREADS.bits()  |
    Permissions::USE_EMBEDDED_ACTIVITIES.bits() | Permissions::USE_SOUNDBOARD.bits()            |
    Permissions::SEND_VOICE_MESSAGES.bits()     
);
//should be like 102235358350913 or something

const DEFAULT_MONARCH_PERMISSIONS: Permissions = Permissions::ADMINISTRATOR;
//8

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (delete_guild, new_guild, old_guild_id, token): (bool, bool, Id<GuildMarker>, String) = match File::open("conf.json") {
        Err(_) => {
            println!("You'll need to save your bot token to run the program automatically. Paste it here and it will get saved (unencrypted!) to conf.json");

            let mut response = String::new();
            
            io::stdin()
                .read_line(&mut response)
                .expect("Something went wrong reading input");

            response = response.trim().to_string();

            (false, true, Id::new(1), response) 
            //default value, will never get touched because delete guild is false
        },
        Ok(f) => {
            let conf: Configuration = serde_json::from_reader(f).expect("Failed to parse JSON. Is everything formatted correctly?");

            println!("It seems you already have a configuration file.");
            println!("Type \"RESTART WITH NEW SERVER\" and the program will attempt to set up a new server after deleting the old one");
            println!("Type \"DELETE CURRENT SERVER\" and the program will try to delete the old one and then halt.");

            let mut response = String::new();

            io::stdin()
                .read_line(&mut response)
                .expect("Something went wrong reading input");

            println!("Response: {response}");

            let new_guild = match response.trim() {
                "RESTART WITH NEW SERVER" => {
                    true
                },
                "DELETE CURRENT SERVER" => {
                    false
                },
                _ => {
                    panic!("Invalid response. Program exiting.");
                }
            };

            (true, new_guild, conf.guild_id, conf.token)
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
                        .delete_guild(old_guild_id)
                        .await?;

                    println!("Response from discord: {discord_response:?}");
                    println!("Destroyed old server!");

                    fs::remove_file("conf.json")?;

                    println!("Old conf file");

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

                // This doesn't return the code
                let new_invite = client.create_invite(new_system_channel_id).max_age(0)?.await?;
                // Get the code from here instead
                let new_channel_id = new_invite.model().await?.channel.expect("oops no channel").id;
                let channel_invites = client.channel_invites(new_channel_id).await?;
                let new_invite_code = channel_invites.model().await?[0].code.to_owned();

                println!("Invite code: discord.gg/{new_invite_code}");

                let monarch_role = client.create_role(new_guild.id)
                    .name("The Monarch")
                    .permissions(DEFAULT_MONARCH_PERMISSIONS)
                    .await?;

                let monarch_role_id = monarch_role.model().await?.id;

                let monarch_user_id = Id::new(1);
                let remaining_monarchs: Vec<Id<UserMarker>> = Vec::new();

                let conf = Configuration {
                    token,
                    guild_id: new_guild.id,
                    monarch_role_id,
                    monarch_user_id,
                    remaining_monarchs,
                    no_ping: false,
                    default_permissions: DEFAULT_PERMISSIONS,
                    default_monarch_permissions: DEFAULT_MONARCH_PERMISSIONS,
                    initial_invite: new_invite_code,
                };

                let j = serde_json::to_string_pretty(&conf)?;

                fs::write("conf.json", j)?;

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