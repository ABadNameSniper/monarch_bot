use base64::encode;
use rand::Rng;
use std::{
    env, 
    process::exit, 
    fs::{
        self, 
        File
    }, 
    io::Read
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
    guild::{
        Permissions,
    }
};
use twilight_http::Client;

const CDN_WEBSITE: &str = "https://cdn.discordapp.com";

//yucky bits stuff. remedy ASAP (not supported in twilight yet)
const DEFAULT_PERMISSIONS: Permissions = Permissions::from_bits_truncate(
    Permissions::CREATE_INVITE.bits() | Permissions::READ_MESSAGE_HISTORY.bits() | Permissions::SPEAK.bits() | Permissions::VIEW_CHANNEL.bits() |
    Permissions::CONNECT.bits() | Permissions::ATTACH_FILES.bits() | Permissions::SEND_MESSAGES.bits() | Permissions::SEND_MESSAGES_IN_THREADS.bits() |
    Permissions::ADD_REACTIONS.bits() | Permissions::CHANGE_NICKNAME.bits() | Permissions::STREAM.bits() | Permissions::CREATE_PUBLIC_THREADS.bits() |
    Permissions::USE_EXTERNAL_STICKERS.bits() | Permissions::USE_EMBEDDED_ACTIVITIES.bits() | Permissions::USE_EXTERNAL_EMOJIS.bits() | 
    Permissions::EMBED_LINKS.bits() | Permissions::USE_VAD.bits()
);


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
                            .permissions(DEFAULT_PERMISSIONS)//i can't figure out how to make it "no permissions"
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
                        //i'm going to assume this just carries on if the member left the gulid already
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
                        //let list = 
                        // ok so like i could filter all members or just keep trying until i pick an eligible one
                        // or i could filter all of them every time. If i do it this way, people who leave during a change
                        // will not be put back on the list until the cycle repeats.
                        serde_json::from_slice::<Vec<Id<UserMarker>>>(&contents)?
                        // .into_iter()
                        // .filter(
                        //     | saved_user_marker| chunk_members.iter().any(
                        //         | guild_member| guild_member.user.id.eq(saved_user_marker)
                        //     )
                        // )
                        // .collect()

                        // ok so maybe like i should just try to give someone admin because it could fail if all users are fetched
                        // and then the future admin leaves immediately after.
                        // yeah, it should probably just try to appoint the monarch and then try again if it fails...

                        //list
                    },
                    Err(_) => {
                        client
                            .create_message(system_channel_id)
                            .content("And so another era of kings and queens has passed. What will this cycle of history bring?")?
                            .await?;
                        let mut all_members = Vec::new();
                        for member in chunk_members {
                            let id = member.user.id;
                            if !member.user.bot {
                                all_members.push(id);
                            }
                        }

                        all_members
                    }
                };

                #[allow(unused)]
                let mut new_monarch_id: Id<UserMarker> = Id::new(0);
                
                let eligible_member_count = filtered_members.len();

                let mut countdown = eligible_member_count;

                loop {

                    let remove_monarch_index = rand::thread_rng().gen_range(0 .. eligible_member_count);
                    
                    new_monarch_id = filtered_members.swap_remove(remove_monarch_index);
                    
                    println!("{:?}", &filtered_members);

                    if !filtered_members.is_empty() {
                        let filtered_members_json = serde_json::to_string(&filtered_members)?;
                        println!("Saving list of everyone else {:?}", {&filtered_members_json});
                        fs::write("remaining_monarchs.json", filtered_members_json)?;
                    } else {
                        fs::remove_file("remaining_monarchs.json")?;
                        println!("Out of monarchs! Will generate new list next cycle!")
                    }

                    match client.add_guild_member_role(guild_id, new_monarch_id, monarch_role_id).await {
                        Ok(res) => {
                            println!("{res:?}, {}", res.status());
                            break;
                        }
                        Err(err) => {
                            println!("{err}");
                            if countdown == 0 {
                                panic!("Could not find a suitable monarch!");
                            }
                            countdown -= 1;
                        }
                    }
                };

                let monarch_id_string = new_monarch_id.get().to_string();
                fs::write("monarch_user_id.txt", &monarch_id_string)?;

                println!("New monarch appointed and ID saved");

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

                //TODO: test animated/no PFP avatars
                let (file_type, image_path) = match guild_member.avatar {
                    Some(avatar) => (if avatar.is_animated() {"gif"} else {"webp"}, format!("guilds/{guild_id}/users/{user_id}/{avatar}")),
                    None => match guild_member.user.avatar {
                        Some(avatar) => (if avatar.is_animated() {"gif"} else {"webp"}, format!("avatars/{user_id}/{avatar}")),
                        None => ("png", format!("embed/avatars/{}", guild_member.user.discriminator % 5)),//as per discord
                    }
                };

                let encoded_image = encode(reqwest::get(format!("{CDN_WEBSITE}/{image_path}.{file_type}")).await?.bytes().await?);
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