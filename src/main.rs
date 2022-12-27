use futures_util::StreamExt;
use twilight_http;
use std::{env, process::exit};
use twilight_gateway::{Event, Intents, Shard};
use twilight_model::{
    gateway::{
        payload::outgoing::UpdatePresence,
        presence::{
            Activity, ActivityType, MinimalActivity, Status
        }
    }, 
    guild::Permissions
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

        // i'm not sure if this is the best way to wait for this, but it's what
        // the example gave me

        match &event {
            Event::Ready(_) => {

                println!("Ready for action!");

                let minimal_activity = MinimalActivity {
                    kind: ActivityType::Playing,
                    name: "Russian roulette but with people instead of bullets".to_owned(),
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

                let new_guild = client
                    .create_guild(String::from("Brand New Server"))
                    .expect("Invalid Name!")
                    .await?
                    .model()
                    .await?;
                    
                let new_system_channel_id = new_guild.system_channel_id.expect("Crap, couldn't get the system channel ID");
                println!("Guild created!");


                // //This works, doesn't return the code, though
                let new_invite = client.create_invite(new_system_channel_id).await?;
                
                let new_channel_id = new_invite.model().await?.channel.expect("oops no channel").id;

                let channel_invites = client.channel_invites(new_channel_id).await?;
                let new_invite_code = &channel_invites.model().await?[0].code;

                println!("Invite code: discord.gg/{new_invite_code}");

                let admin_permission = Permissions::ADMINISTRATOR;

                let admin_role = client.create_role(new_guild.id)
                    .name("The Administrator")
                    .permissions(admin_permission)
                    .await?;

                let admin_role_id = admin_role.model().await?.id;

                let all_role_ids = vec![admin_role_id];

                //TODO
                // wait for everyone to join the server, have slash command to start the administration
                // (destroy slash command after use)
                // include options like period of role changing

                println!("Administrator role created. Waiting for first person to join");

                while let Some(event) = events.next().await {
                    match &event {
                        Event::MemberAdd(member) => {
                            let _member = client
                            .update_guild_member(
                                new_guild.id, 
                                member.user.id
                            )
                            .roles(&all_role_ids)
                            .await?
                            .model()
                            .await?;

                            //TODO this should be running like 24/7 and changing the admin periodically
                            println!("Administrator assigned, quitting program");
                            exit(0);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(())
}