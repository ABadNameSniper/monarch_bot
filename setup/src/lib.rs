
use twilight_model::{
    id::{
        Id,
        marker::{
            GuildMarker,
            RoleMarker,
            UserMarker
        },
    },
    guild::Permissions
};
use serde::{
    Serialize, 
    Deserialize
};

#[derive(Serialize, Deserialize)]

pub struct Configuration {
    pub token: String,
    pub guild_id: Id<GuildMarker>,
    pub monarch_role_id: Id<RoleMarker>,
    pub monarch_user_id: Id<UserMarker>,
    pub remaining_monarchs: Vec<Id<UserMarker>>,
    pub no_ping: bool,
    pub default_permissions: Permissions,
    pub initial_invite: String,
}