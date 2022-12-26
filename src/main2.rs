use futures_util::StreamExt;
use twilight_cache_inmemory::InMemoryCache;
use std::env;
use twilight_gateway::{Intents, Shard};
use twilight_http::Client;
use twilight_model::id::Id;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let intents = Intents::GUILD_MESSAGES | Intents::DIRECT_MESSAGES;
    let token = env::var("DISCORD_TOKEN")?;
    let http = Client::new(token.to_owned());
    let (shard, mut events) = Shard::new(token, intents);
    
    let cache = InMemoryCache::new();

    //let cache_and_http = cache.update();

    //let user_id = UserId

    //let guild_ids: Vec = http.guild().await.expect("failed to get guilds");

    let current_user = http.current_user().await.unwrap();

   

    shard.start().await?;
    println!("Created shard");

    while let Some(event) = events.next().await {
        println!("Event: {event:?}");
    }

    Ok(())
}