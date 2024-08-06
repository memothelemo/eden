CREATE TABLE guild_settings (
    "id" BIGINT PRIMARY KEY NOT NULL,
    "created_at" TIMESTAMP WITHOUT TIME ZONE
        NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "updated_at" TIMESTAMP,
    "data" JSONB NOT NULL
);
SELECT manage_updated_at('guild_settings');
