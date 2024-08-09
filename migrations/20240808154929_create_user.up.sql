CREATE TABLE "user" (
    "id" BIGINT PRIMARY KEY,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "updated_at" TIMESTAMP,

    "developer_mode" BOOLEAN NOT NULL DEFAULT false
);
SELECT manage_updated_at('user');
