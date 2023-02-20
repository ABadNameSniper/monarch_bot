use base64::encode;
use rand::Rng;
use twilight_http;
use std::{env, process::exit, fs::{self, File}, io::Read};
use twilight_gateway::{Event, Intents, Shard, ShardId};
use twilight_model::{
    gateway::{
        payload::outgoing::{UpdatePresence, RequestGuildMembers},
        presence::{
            Activity, ActivityType, MinimalActivity, Status
        }
    }, 
    id::{Id, marker::{UserMarker, GuildMarker}}, guild::Permissions,
};
use twilight_http::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN")?;

    let mut shard = Shard::new(
        ShardId::ONE,
        token.to_owned(),
        Intents::GUILD_MEMBERS | Intents::GUILDS
    );

    let client = Client::new(token.to_owned());

    loop { let event = match shard.next_event().await {
        Ok(event) => match &event {
            Event::GuildCreate(guild) => {

                shard
                    .command(&RequestGuildMembers::builder(guild.id).query("", None))
                    .await?;

                println!("bah humbug");
                // let guild_members = guild.members.to_owned();
                // println!("{guild_members:?}");

                
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
            Event::MemberChunk(chunk) => {
                let chunk_members = chunk.to_owned().members;
                println!("Recieved members!");

                let guild_id: Id<GuildMarker> = fs::read_to_string("guild_id.txt")?.parse()?;
                println!("Got server and role IDs");

                let guild = client
                    .guild(guild_id)
                    .await?
                    .model()
                    .await?;

                let roles_vec = guild.roles;

                let default_permissions: Permissions = 
                Permissions::CREATE_INVITE | Permissions::READ_MESSAGE_HISTORY | Permissions::SPEAK | Permissions::VIEW_CHANNEL |
                Permissions::CONNECT | Permissions::ATTACH_FILES | Permissions::SEND_MESSAGES | Permissions::SEND_MESSAGES_IN_THREADS |
                Permissions::ADD_REACTIONS | Permissions::CHANGE_NICKNAME | Permissions::STREAM | Permissions::CREATE_PUBLIC_THREADS |
                Permissions::USE_EXTERNAL_STICKERS | Permissions::USE_EMBEDDED_ACTIVITIES | Permissions::USE_EXTERNAL_EMOJIS | 
                Permissions::EMBED_LINKS | Permissions::USE_VAD;

                //consider giving @everyone the default permissions, while removing all permissions from other roles.
                //would that work? would that even be useful? i have no idea -- but that is an idea!

                let monarch_role_id = Id::new(
                    fs::read_to_string("monarch_role_id.txt").expect("Couldn't read monarch role id file!").parse()?
                );                

                println!("Guild permissions: {:?}", guild.permissions);

                for role in roles_vec {
                    if role.id.ne(&monarch_role_id) {
                        println!("{} with id {}", role.name, role.id);
                        client
                            .update_role(guild_id, role.id)
                            .permissions(default_permissions)//i can't figure out how to make it "no permissions"
                            .await?;                    //so i'll leave it with just this one, the default.    
                        //this shouldn't wait to be syncrhonous, but i can't figure out how to get things to work without doing this
                        //wasn't there like a command function?
                        //anyway making this part asynchronous is on the TODO list.

                        
                    }
                    
                }

                /*
                Two ideas: set id to 0000000001 or something that will never evaluate to true OR
                two functions for determining new monarch based on whether there was an old one found.
                */
                
                match fs::read_to_string("monarch_user_id.txt") {
                    Ok(result) => {
                        client.remove_guild_member_role(guild_id, Id::new(result.parse()?), monarch_role_id).await?;

                        println!("Removed old monarch");
                        fs::remove_file("monarch_user_id.txt")?;
                    },
                    Err(_error) => ()
                };

                let system_channel_id = guild
                    .system_channel_id
                    .expect("No system channel? Is that even possible?");

                let mut filtered_members: Vec<Id<UserMarker>> = match File::open("remaining_monarchs.json") {
                    Ok(mut file) => {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents).expect("Error reading file");
                        let list = serde_json::from_slice::<Vec<Id<UserMarker>>>(&contents)?;
                        list
                    },
                    Err(_) => {
                        client
                            .create_message(system_channel_id)
                            .content("And so another era of kings and queens has passed. What will this cycle of history bring?")?
                            .await?;
                        let mut all_members = Vec::new();
                        for member in chunk_members {
                            let id = member.user.id;
                            if member.user.bot == false {
                                all_members.push(id);
                            }
                        }

                        all_members
                    }
                };

                let remove_monarch_index = rand::thread_rng().gen_range(0..filtered_members.len());
                
                let new_monarch_id = filtered_members.swap_remove(remove_monarch_index);
                
                println!("{:?}", &filtered_members);

                if filtered_members.len() > 0 {
                    let filtered_members_json = serde_json::to_string(&filtered_members)?;
                    println!("Saving list of everyone else {:?}", {&filtered_members_json});
                    fs::write("remaining_monarchs.json", filtered_members_json)?;
                } else {
                    fs::remove_file("remaining_monarchs.json")?;
                    println!("Out of monarchs! Will generate new list next cycle!")
                }


                let monarch_id_string = new_monarch_id.get().to_string();
                fs::write("monarch_user_id.txt", &monarch_id_string)?;
                /*let partial = */client.add_guild_member_role(
                    guild_id,
                    new_monarch_id,
                    monarch_role_id
                ).await?;
                println!("New monarch appointed and ID saved");
                //can't i just use this partial?
                //println!("{partial:?}");

                client
                    .create_message(system_channel_id)
                    .content(&format!(
                        "<@{monarch_id_string}>, you are the Monarch for today!"
                    ))?
                    .await?;
                println!("Pinged monarch in system channel");

                let guild_member = client
                    .guild_member(guild_id, new_monarch_id)
                    .await?.model().await?;

                println!("{guild_member:?}");

                let user_id = new_monarch_id.get().to_string();

                //through testing i can confirm non-animated user avatars work.
                let (file_type, image_url) = match guild_member.avatar {
                    Some(avatar) => ("webp", format!("https://cdn.discordapp.com/guilds/{guild_id}/users/{user_id}/{avatar}.webp")),
                    None => match guild_member.user.avatar {
                        Some(avatar) => ("webp", format!("https://cdn.discordapp.com/avatars/{user_id}/{avatar}.webp")),
                        None => ("png", format!("https://cdn.discordapp.com/embed/avatars/{}.png", guild_member.user.discriminator.to_string())),
                    }
                };

                //let encoded_image = encode(reqwest::blocking::get(image_url)?.bytes()?);
                let encoded_image = encode(reqwest::get(image_url).await?.bytes().await?);
                //hopefully setting the file type makes it work with unset profile pictures. i'll still need to do testing with .gifs...
                let icon = format!("data:image/{file_type};base64,{encoded_image}");

                client.update_guild(guild_id).icon(Some(&icon)).await?;

                println!("Server icon updated");

                exit(0);
            }
            other => {println!("other thing: {other:?}")}

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