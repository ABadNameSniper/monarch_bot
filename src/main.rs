use futures_util::StreamExt;
use twilight_http;
use std::env;
use twilight_gateway::{Event, Intents, Shard};
use twilight_model::{
    gateway::{
        payload::outgoing::{
            UpdatePresence,
            RequestGuildMembers
        },
        presence::{
            Activity, ActivityType, MinimalActivity, Status
        }
    }, 
    id::Id,
};


use twilight_http::Client;

//::new(&client, String::from("Epic New Server"));

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

        println!("Event: {event:?}");

        match &event {
            Event::GuildCreate(guild) => {
                // Let's request all of the guild's members for caching.
                //println!("Cool guild made. {event:?}");

                shard
                    .command(&RequestGuildMembers::builder(guild.id).query("", None))
                    .await?;
            }
            Event::Ready(_) => {

                //::new(&client, String::from("Epic New Server"));
                
                let minimal_activity = MinimalActivity {
                    kind: ActivityType::Custom,
                    name: "running on twilight".to_owned(),
                    url: None,
                };
                let command = UpdatePresence::new(
                    Vec::from([Activity::from(minimal_activity)]),
                    false,
                    Some(1),
                    Status::Online,
                )?;

                shard.command(&command).await?;

                println!("Time for a new guild!");

                //To-Do:
                //pay attention to this, get the ID, make the invite immediately after this.
                let new_guild = client.create_guild(String::from("Neat New Server")).expect("Invalid Name!").await?;

                println!("New guild created?");

                //new_guild.add_role();

                //shard.command(&new_guild);

            }
            _ => {
                
            }
        }
    }

    Ok(())
}