use futures_util::StreamExt;
use base64::encode;
use rand::Rng;
use twilight_http;
use std::{env, process::exit, fs::{self, File}, io::Read};
use twilight_gateway::{Event, Intents, Shard};
use twilight_model::{
    gateway::{
        payload::outgoing::{UpdatePresence, RequestGuildMembers},
        presence::{
            Activity, ActivityType, MinimalActivity, Status
        }
    }, 
    id::{Id, marker::{UserMarker, GuildMarker, RoleMarker}},
};
use twilight_http::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN")?;
    // To interact with the gateway we first need to connect to it (with a shard or cluster)
    let (shard, mut events) = Shard::new(
        token.to_owned(),
        Intents::GUILD_MEMBERS | Intents::GUILDS,
    );

    let client = Client::new(token.to_owned());

    shard.start().await?;
    println!("Created shard");

    while let Some(event) = events.next().await {
        match &event {
            Event::GuildCreate(guild) => {

                shard
                    .command(&RequestGuildMembers::builder(guild.id).query("", None))
                    .await?;

                println!("bah humbug");
                let guild_members = guild.members.to_owned();
                println!("{guild_members:?}");

                
            }
            Event::Ready(_) => {

                println!("Ready for action!");

                let minimal_activity = MinimalActivity {
                    kind: ActivityType::Playing,
                    name: "Replacing the administrator...".to_owned(),
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

                //let mut filtered_members: Vec<Id<UserMarker>> = Vec::new();

                let guild_id: Id<GuildMarker> = fs::read_to_string("guild_id.txt")?.parse()?;
                let admin_role_id: Id<RoleMarker> = fs::read_to_string("admin_role_id.txt")?.parse()?;
                println!("Got server and role IDs");


                /*
                Two ideas: set id to 0000000001 or something that will never evaluate to true OR
                two functions for determining new admin based on whether there was an old one found.
                */
                match fs::read_to_string("administrator_id.txt") {
                    Ok(result) => {
                        // client.remove_guild_member_role(
                        //     guild_id,
                        //     Id::new(result.parse()?),
                        //     admin_role_id
                        // ).await?;

                        //yes Gabe, I had to remove *all* roles from the previous administrator because of you
                        //shoutouts for finding a security flaw i guess!

                        //update: i'm just going to have to strip all roles of all permissions to make roles cosmetic. (except The Administrator role)
                        //maybe even have to remove all channel role permissions. yikes. i'll do that later.
                        let no_roles: Vec<Id<RoleMarker>> = vec![];

                        client.update_guild_member(
                            guild_id, Id::new(result.parse()?)
                        )
                        .roles(&no_roles)
                        .await?;

                        println!("Removed old administrator");
                        fs::remove_file("administrator_id.txt")?;
                    },
                    Err(_error) => ()
                };

                let system_channel_id = client
                    .guild(guild_id)
                    .await?
                    .model()
                    .await?
                    .system_channel_id
                    .expect("No system channel? Is that even possible?");

                let mut filtered_members: Vec<Id<UserMarker>> = match File::open("remaining_admins.json") {
                    Ok(mut file) => {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents).expect("Error reading file");
                        let list = serde_json::from_slice::<Vec<Id<UserMarker>>>(&contents)?;
                        list
                    },
                    Err(_) => {
                        client
                            .create_message(system_channel_id)
                            .content("The Administration cycle has been completed. Now everyone will get another turn.")?
                            .await?;
                        let mut all_members = Vec::new();
                        for member in chunk_members {
                            let id = member.user.id;
                            if member.user.bot == false {
                                all_members.push(id);
                            }
                        }
                        // let all_members_json = serde_json::to_string(&all_members).unwrap();
                        // fs::write("remaining_admins.json", all_members_json).unwrap();
                        all_members
                    }
                };

                let remove_admin_index = rand::thread_rng().gen_range(0..filtered_members.len());
                
                let new_admin_id = filtered_members[remove_admin_index];
                
                let filtered_members_json = serde_json::to_string(&filtered_members.swap_remove(remove_admin_index))?;
                if filtered_members.len() > 0 {
                    fs::write("remaining_admins.json", filtered_members_json)?;
                    println!("Saved list of everyone else");
                } else {
                    println!("Out of admins! Will generate new list next cycle!")
                }

                
                let admin_id_string = new_admin_id.get().to_string();
                fs::write("administrator_id.txt", &admin_id_string)?;
                /*let partial = */client.add_guild_member_role(
                    guild_id,
                    new_admin_id,
                    admin_role_id
                ).await?;
                println!("New administrator appointed and ID saved");
                //can't i just use this partial?
                //println!("{partial:?}");

                client
                    .create_message(system_channel_id)
                    .content(&format!(
                        "<@{admin_id_string}>, you are the Administrator for today!"
                    ))?
                    .await?;
                println!("Pinged administrator in system channel");

                let guild_member = client
                    .guild_member(guild_id, new_admin_id)
                    .await?.model().await?;

                println!("{guild_member:?}");

                let user_id = new_admin_id.get().to_string();

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
        }
    }
    Ok(())
}