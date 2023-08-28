use base64::encode;
use rand::Rng;
use std::{
    fs::{
        File, self
    }, 
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
        marker::UserMarker
    }, 
};
use twilight_http::Client;

const CDN_WEBSITE: &str = "https://cdn.discordapp.com";

use setup::Configuration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    let conf: Configuration = serde_json::from_reader(
        File::open("conf.json").unwrap()
    ).expect("Invalid JSON! Is everything formatted correctly?");

    let Configuration {
        token,
        guild_id,
        monarch_role_id,
        monarch_user_id,
        remaining_monarchs,
        no_ping,
        default_permissions,
        initial_invite
    } = conf;

    let mut shard = Shard::new(
        ShardId::ONE,
        token.to_owned(),
        Intents::GUILD_MEMBERS | Intents::GUILDS
    );

    let client = Arc::new(
        Client::new(token.to_owned())
    );

     // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    loop { let event = match shard.next_event().await {
        Ok(event) => match &event {
            Event::GuildCreate(guild) => {
                shard
                    //Not sure if this scales!
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
                let command = UpdatePresence::new(
                    Vec::from([Activity::from(minimal_activity)]),
                    false,
                    Some(1),
                    Status::Online,
                )?;

                //This will probably not significantly negatively impact delays...
                shard.command(&command).await.expect("Something went wrong setting the status!");
                println!("Status set!"); 
            }
            
            //This is from querying everyone up in the GuildCreate event match
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
                
                let client_arc = client.clone();
                let old_monarch_removed = tokio::spawn(async move {
                    match client_arc.remove_guild_member_role(
                            guild_id, 
                            monarch_user_id, 
                            monarch_role_id
                        ).await 
                    {
                        Ok(_) => {
                            println!("Removed old monarch");
                        },
                        Err(err) => {
                            //If 404, then continue as normal? It means they've left the server
                            //also maybe add a thing in the json to show if first time
                            println!("Could not remove old monarch. Reason: {err}")
                        }
                    };
                    
                });

                let mut filtered_ids: Vec<Id<UserMarker>> = remaining_monarchs
                    .iter()
                    .filter(|remaining_eligible_id| eligible_ids.contains(&remaining_eligible_id))
                    .cloned()
                    .collect();

                let mut new_cycle = false;

                if filtered_ids.is_empty() {
                    filtered_ids = eligible_ids;
                    new_cycle = true;
                }
            
                let monarch_user_id = filtered_ids.swap_remove(
                    rand::thread_rng().gen_range(0 .. filtered_ids.len())
                );
                
                println!("Selected Monarch: {:?}", &monarch_user_id);
                println!("{:?}", &filtered_ids);

                //could probably just check for 404 errors instead...
                if !client.add_guild_member_role(guild_id, monarch_user_id, monarch_role_id)
                    .await?
                    .status()
                    .is_success()
                {
                    panic!("Failed to appoint monarch");
                }

                //Might as well save now!
                tokio::try_join!(old_monarch_removed)?;

                let conf = Configuration {
                    token,
                    guild_id,
                    monarch_role_id,
                    monarch_user_id,
                    remaining_monarchs: filtered_ids,
                    no_ping,
                    default_permissions,
                    initial_invite
                };

                let j = serde_json::to_string_pretty(&conf)?;

                match fs::write("conf.json", j) {
                    Ok(_) => {
                        println!("Saved new dynastical information!");
                    }
                    Err(_) => {
                        println!("Could not save new information to the file!");
                        println!("Remember, the appointed monarch should have the Id: {}", monarch_user_id);
                    }
                };

                let guild_member = client
                    .guild_member(guild_id, monarch_user_id)
                    .await?
                    .model()
                    .await?;

                println!("Selected member: {guild_member:?}");


                let user = guild_member.user;


                let user_id_string = monarch_user_id.get().to_string();

                let (file_type, image_path) = match guild_member.avatar {
                    Some(avatar) => (
                        if avatar.is_animated() 
                            {"gif"} 
                        else 
                            {"webp"}, 
                        format!("guilds/{guild_id}/users/{user_id_string}/avatars/{avatar}")
                    ),
                    None => match user.avatar {
                        Some(avatar) => (
                            if avatar.is_animated() 
                                {"gif"} 
                            else 
                                {"webp"}, 
                            format!("avatars/{user_id_string}/{avatar}")
                        ),
                        None => (
                            "png", 
                            format!("embed/avatars/{}", user.discriminator % 5)
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

                let monarch_id_string = monarch_user_id.to_string();

                let final_announcement_string = format!(
                    "<@{monarch_id_string}>, you are the new Monarch!"
                );

                if no_ping {
                    let display_name: String = match guild_member.nick {
                        Some(nick) => nick,
                        None => {
                            user.name
                        }
                    };
                    let message_id = client
                        .create_message(system_channel_id)
                        .content(&format!(
                            "{}, you are the new Monarch!", display_name
                        ))?
                        .await?
                        .model()
                        .await?
                        .id;

                    client
                        .update_message(system_channel_id, message_id)
                        .content(Some(&final_announcement_string))? //not sure why update_message requires an Option while create_message doesn't!
                        .await?;
                } else {
                    client
                        .create_message(system_channel_id)
                        .content(&final_announcement_string)?
                        .await?;
                }
                
                println!("Declared monarch in system channel");

                let roles_vec = guild.roles;

                println!("Guild permissions: {:?}", guild.permissions);

                //consider giving @everyone the default permissions, while removing all permissions from other roles.
                //would that work? would that even be useful? i have no idea -- but that is an idea!

                let default_roles_tasks = roles_vec
                    .into_iter()
                    .filter(|role| role.id.ne(&monarch_role_id))
                    .map(|role| {
                        let client_arc = client.clone();

                        tokio::spawn(async move {
                            println!("{} with id {}", role.name, role.id);

                            let result = client_arc
                                .update_role(guild_id, role.id)
                                .permissions(default_permissions)
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