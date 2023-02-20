use twilight_http;
use std::{env, process::exit, fs, fs::*};
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
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let token = env::var("DISCORD_TOKEN")?;
    // To interact with the gateway we first need to connect to it (with a shard or cluster)

    let mut shard = Shard::new(
        ShardId::ONE,
        token.to_owned(),
        Intents::GUILD_MEMBERS | Intents::GUILDS,
    );

    let mut client = Client::new(token.to_owned());

    loop { let event = match shard.next_event().await {
        Ok(event) => match &event {
            Event::Ready(_) => {

                println!("Ready for action!");

                let minimal_activity = MinimalActivity {
                    kind: ActivityType::Playing,
                    name: "Setting up a brand new server!".to_owned(),
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

                let _new_file = match File::options().read(true).write(true).create_new(true).open("guild_id.txt") {
                    Ok(file) => file,
                    Err(_) => {
                        let server_id: u64 = fs::read_to_string("guild_id.txt")?.parse().expect("Couldn't read guild_id.txt");

                        println!("Looks like you already have a server. Press enter to continue or \"RESTART WITH NEW SERVER\" to restart with a new server.");
            
                        let mut response = String::new();
            
                        io::stdin()
                            .read_line(&mut response)
                            .expect("Something went wrong reading input");

                        println!("Response: {response}");

                        if response.trim() == "RESTART WITH NEW SERVER" {
                            client = delete_old_server(client, &server_id).await?;//this is dumb
                            //how can i do this without client being mutable
                            //the compiler complains because client gets used previously in the loop...
                            //but this isn't really a loop. the program is guaranteed to end at this point?

                            //wait no the above is false

                            File::options().read(true).write(true).create_new(true).open("guild_id.txt")? //better not error pls
                        } else if response.trim() == "DELETE CURRENT SERVER" {
                            delete_old_server(client, &server_id).await?;

                            exit(0);
                        } else {
                            println!("Alright, stopping setup execution.");
                            exit(0);
                        }
                    }
                };

                let new_guild = client
                    .create_guild(String::from("Brand New Server"))
                    .expect("Invalid Name!")
                    .await?
                    .model()
                    .await?;
                    
                let new_system_channel_id = new_guild.system_channel_id.expect("Couldn't get the system channel ID");
                println!("Guild created!");

                //save the server id
                //new_file.write_all(&new_guild.id.get().to_string())?;
                //idk how to work with the file struct thingies so we're doing it this way I guess
                fs::write("guild_id.txt", new_guild.id.get().to_string())?;
                println!("Guild ID saved");

                // This doesn't return the code
                let new_invite = client.create_invite(new_system_channel_id).await?;
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

                //TODO
                // wait for everyone to join the server, have slash command to start the monarchistration
                // (destroy slash command after use)
                // include options like period of role changing

                println!("Monarch role created. Waiting for first person to join");

                fs::write("monarch_role_id.txt", monarch_role_id.get().to_string())?;
                println!("Admin role saved to file");

                //
                exit(0);

                // // I'll call removing this a good thing. I mean, why should the first person
                // // to join just get owner permissions?
                // // definitely not because idk how to implement this feature now
                // while let Some(event) = events.next().await {
                //     match &event {
                //         Event::MemberAdd(member) => {
                //             let _member = client.add_guild_member_role(
                //                 new_guild.id, 
                //                 member.user.id,
                //                 monarch_role_id
                //             ).await;

                //             fs::write("monarch_user_id.txt", member.user.id.to_string())?;

                //             println!("Administrator assigned and saved to file, quitting program");
                //             exit(0);
                //         }
                //         _ => {}
                //     }
                // }
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

async fn delete_old_server(client: Client, &server_id: &u64) -> anyhow::Result<Client> {
    let discord_response = client
    .delete_guild(Id::new_checked(server_id).unwrap())
    .await?;

    println!("Response from discord: {discord_response:?}");
    println!("Destroyed old server!");

    fs::remove_file("guild_id.txt")?;
    fs::remove_file("monarch_user_id.txt")?;
    fs::remove_file("monarch_role_id.txt")?;

    println!("Deleted records of old server");

    Ok(client)
}