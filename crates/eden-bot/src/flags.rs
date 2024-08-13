use twilight_cache_inmemory::ResourceType;
use twilight_gateway::{EventTypeFlags, Intents};

pub const CACHE_RESOURCE_TYPES: ResourceType = ResourceType::GUILD
    .union(ResourceType::USER)
    .union(ResourceType::USER_CURRENT)
    .union(ResourceType::CHANNEL);

pub const INTENTS: Intents = Intents::GUILDS
    .union(Intents::DIRECT_MESSAGES)
    .union(Intents::GUILD_MEMBERS)
    .union(Intents::GUILD_MESSAGES)
    .union(Intents::MESSAGE_CONTENT);

pub const FILTERED_EVENT_TYPES: EventTypeFlags = EventTypeFlags::READY
    .union(EventTypeFlags::RESUMED)
    .union(EventTypeFlags::INTERACTION_CREATE)
    .union(EventTypeFlags::DIRECT_MESSAGES)
    .union(EventTypeFlags::GUILD_CREATE);
