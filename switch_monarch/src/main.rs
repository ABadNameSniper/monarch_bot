use base64::encode;
use rand::Rng;
use std::{
    fs::{
        self, 
        File
    }, 
    io::Read, 
    sync::Arc
};
use twilight_gateway::{
    Event, 
    Intents, 
    Shard, 
    ShardId
};
use twilight_model::{
    gateway::{
        payload::outgoing::{
            UpdatePresence, 
            RequestGuildMembers
        },
        presence::{
            Activity, 
            ActivityType, 
            MinimalActivity, 
            Status
        }
    }, 
    id::{
        Id, 
        marker::{
            UserMarker, 
            GuildMarker
        }
    }, 
    guild::Permissions
};
use twilight_http::Client;

const CDN_WEBSITE: &str = "https://cdn.discordapp.com";

//TODO: remove function, make it better, then organize code
// sounds oxymoronic... hmmm.

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


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token = fs::read_to_string("bot_token.txt")?;

    let mut shard = Shard::new(
        ShardId::ONE,
        token.to_owned(),
        Intents::GUILD_MEMBERS | Intents::GUILDS
    );

    let client = Client::new(token.to_owned());

    let client = Arc::new(client);

    loop { let event = match shard.next_event().await {
        Ok(event) => match &event {
            Event::GuildCreate(guild) => {
                shard
                    .command(&RequestGuildMembers::builder(guild.id).query("", None))
                    .await?;
                //perhaps threading here to have access to the guild?
            }
            Event::Ready(_) => {

                println!("Ready for action!");

                let minimal_activity = MinimalActivity {
                    kind: ActivityType::Playing,
                    name: "Replacing the monarch...".to_owned(),
                    url: None,
                };
                let _command = UpdatePresence::new(
                    Vec::from([Activity::from(minimal_activity)]),
                    false,
                    Some(1),
                    Status::Online,
                )?;

            }
            //Not sure if this scales!
            Event::MemberChunk(chunk) => {
                let eligible_ids: Vec<Id<UserMarker>> = chunk
                    .to_owned()
                    .members
                    .into_iter()
                    .filter(|member| !member.user.bot)
                    .map(|member| member.user.id)
                    .collect();
                if eligible_ids.is_empty() {
                    panic!("No eligible monarchs!");
                }
                println!("Recieved members!");

                let guild_id: Id<GuildMarker> = fs::read_to_string("guild_id.txt")?.parse()?;
                println!("Got server and role IDs");

                //consider giving @everyone the default permissions, while removing all permissions from other roles.
                //would that work? would that even be useful? i have no idea -- but that is an idea!

                let monarch_role_id = Id::new(
                    fs::read_to_string("monarch_role_id.txt")
                        .expect("Couldn't read monarch role id file!")
                        .parse()?
                );                

                
                let client_arc = client.clone();
                let old_monarch_removed = tokio::spawn(async move {
                    match fs::read_to_string("monarch_user_id.txt") {
                        Ok(result) => {
                            //i'm going to assume this just carries on if the member left the gulid already
                            match 
                                client_arc.remove_guild_member_role(
                                    guild_id, 
                                    Id::new(result.parse().expect("Could not parse result of file!")), 
                                    monarch_role_id
                                ).await 
                            {
                                Ok(_) => {
                                    println!("Removed old monarch");
                                },
                                Err(err) => {
                                    //If 404, then continue as normal? It means they've left the server
                                    println!("Could not remove old monarch. Reason: {err}")
                                }
                            };
                            fs::remove_file("monarch_user_id.txt").expect("Could not remove old monarch user id file!");
                        },
                        Err(_error) => ()
                    };
                });


                let mut remamining_monarchs_file = File::open("remaining_monarchs.json").unwrap();
                let mut remaining_monarchs_contents: Vec<u8> = Vec::new();
                remamining_monarchs_file.read_to_end(&mut remaining_monarchs_contents).unwrap();

                let saved_ids = serde_json::from_slice::<Vec<Id<UserMarker>>>(&remaining_monarchs_contents)
                    .unwrap();

                let mut filtered_ids: Vec<Id<UserMarker>> = saved_ids
                    .iter()
                    .filter(|remaining_eligible_id| eligible_ids.contains(&remaining_eligible_id))
                    .cloned()
                    .collect();

                let mut new_cycle = false;

                if filtered_ids.is_empty() {
                    filtered_ids = eligible_ids;
                    new_cycle = true;
                }
            
                let new_monarch_id = filtered_ids.swap_remove(
                    rand::thread_rng().gen_range(0 .. filtered_ids.len())
                );
                
                println!("Selected Monarch: {:?}", &new_monarch_id);
                println!("{:?}", &filtered_ids);

                //could probably just check for 404 errors instead...
                if !client.add_guild_member_role(guild_id, new_monarch_id, monarch_role_id)
                    .await?
                    .status()
                    .is_success()
                {
                    panic!("Failed to appoint monarch");
                }

                //Ready to save new monarch
                tokio::try_join!(old_monarch_removed)?;

                let filtered_members_json = serde_json::to_string(&filtered_ids)?;
                println!("Saving list of remaining candidates {:?}", {&filtered_members_json});
                fs::write("remaining_monarchs.json", filtered_members_json)?;

                let monarch_id_string = new_monarch_id.get().to_string();
                fs::write("monarch_user_id.txt", &monarch_id_string)?;
                println!("New monarch appointed and ID saved");


                let guild_member = client
                    .guild_member(guild_id, new_monarch_id)
                    .await?
                    .model()
                    .await?;

                println!("{guild_member:?}");

                let user_id = new_monarch_id.get().to_string();

                let (file_type, image_path) = match guild_member.avatar {
                    Some(avatar) => (
                        if avatar.is_animated() 
                            {"gif"} 
                        else 
                            {"webp"}, 
                        format!("guilds/{guild_id}/users/{user_id}/avatars/{avatar}")
                    ),
                    None => match guild_member.user.avatar {
                        Some(avatar) => (
                            if avatar.is_animated() 
                                {"gif"} 
                            else 
                                {"webp"}, 
                            format!("avatars/{user_id}/{avatar}")
                        ),
                        None => (
                            "png", 
                            format!("embed/avatars/{}", guild_member.user.discriminator % 5)
                        ),//as per discord
                    }
                };
                let encoded_image = encode(
                    reqwest::get(format!("{CDN_WEBSITE}/{image_path}.{file_type}"))
                        .await?
                        .bytes()
                        .await?
                );

                let icon = format!("data:image/{file_type};base64,{encoded_image}");

                client.update_guild(guild_id).icon(Some(&icon)).await?;

                println!("Server icon updated");

                let guild = client
                    .guild(guild_id)
                    .await?
                    .model()
                    .await?;

                let system_channel_id = guild.system_channel_id.unwrap();

                if new_cycle {
                    client
                        .create_message(system_channel_id)
                        .content(
                            "And so another era of kings and queens has passed. What will this cycle of history bring?"
                        )?
                        .await?;
                }

                client
                    .create_message(system_channel_id)
                    .content(&format!(
                        "<@{monarch_id_string}>, you are the Monarch for today!"
                    ))?
                    .await?;
                println!("Pinged monarch in system channel");

                let roles_vec = guild.roles;

                println!("Guild permissions: {:?}", guild.permissions);

                let default_roles_tasks = roles_vec
                    .into_iter()
                    .filter(|role| role.id.ne(&monarch_role_id))
                    .map(|role| {
                        let client_arc = client.clone();

                        tokio::spawn(async move {
                            println!("{} with id {}", role.name, role.id);

                            let result = client_arc
                                .update_role(guild_id, role.id)
                                .permissions(DEFAULT_PERMISSIONS)
                                .await;

                            if let Err(e) = result {
                                eprintln!("Error updating role: {:?}", e);
                            }
                        })
                    });

                for thread in default_roles_tasks {
                    //Probably unecessary! Where is try_join_all?
                    tokio::try_join!(thread)?;
                }

                break;

            }
            other => {
                println!("other thing: {other:?}")
            }

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