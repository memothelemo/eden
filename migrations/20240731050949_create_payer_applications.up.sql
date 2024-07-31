CREATE TABLE payer_applications (
    "id" UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    "created_at" TIMESTAMP WITHOUT TIME ZONE
        NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "updated_at" TIMESTAMP,

    "name" VARCHAR(50) NOT NULL,
    "user_id" BIGINT UNIQUE NOT NULL,

    "java_username" VARCHAR(100) UNIQUE NOT NULL,
    "bedrock_username" VARCHAR(100) UNIQUE,

    "accepted" BOOLEAN,
    "answer" TEXT NOT NULL,
    -- I'm not going to add admin_id because I don't want to
    -- hurt let admin (who rejected the applicant)'s feelings
    "deny_reason" TEXT,

    CONSTRAINT non_empty_name CHECK(length("name") > 0),
    CONSTRAINT answer_length_check CHECK(length("answer") >= 2 AND length("answer") <= 5000),
    CONSTRAINT deny_reason_length_check CHECK(length("deny_reason") >= 2 AND length("deny_reason") <= 5000)
);
SELECT manage_updated_at('payer_applications');

CREATE OR REPLACE FUNCTION check_identity_if_not_occupied_from_payers_application()
    RETURNS TRIGGER
    AS $$
DECLARE
    "exists" BOOLEAN = FALSE;
BEGIN
    SELECT EXISTS(
        SELECT * FROM payer_applications
        WHERE "java_username" = NEW.name
            OR "bedrock_username" = NEW.name
    ) INTO "exists";

    IF ("exists" = TRUE) THEN
        RAISE EXCEPTION 'username % is already occupied', NEW.name
        USING ERRCODE = "23505";
    END IF;

    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER check_identity_if_not_occupied_from_payers_application
BEFORE INSERT ON identities
FOR EACH ROW EXECUTE PROCEDURE check_identity_if_not_occupied_from_payers_application();

CREATE OR REPLACE FUNCTION check_payer_application()
    RETURNS TRIGGER
    AS $$
DECLARE
    "java_username_exists" BOOLEAN = FALSE;
    "bedrock_username_exists" BOOLEAN = FALSE;
    "is_already_registered" BOOLEAN = FALSE;
BEGIN
    SELECT EXISTS(
        SELECT * FROM identities
        WHERE "name" = NEW.java_username
    ) INTO "java_username_exists";

    IF (NEW.bedrock_username IS NULL) THEN
        SELECT EXISTS(
            SELECT * FROM identities
            WHERE "name" = NEW.bedrock_username
        ) INTO "bedrock_username_exists";
    END IF;

    IF ("java_username_exists" = TRUE OR "bedrock_username_exists" = TRUE) THEN
        RAISE EXCEPTION 'Java or Bedrock username is occupied'
        USING ERRCODE = "23505";
    END IF;

    -- Make sure the "user_id" is not already registered yet!
    SELECT EXISTS(
        SELECT * FROM payers
        WHERE "id" = NEW.user_id
    ) INTO "is_already_registered";

    IF ("is_already_registered" = TRUE) THEN
        RAISE EXCEPTION 'User % is already registered', NEW.id
            USING ERRCODE = "23505";
    END IF;

    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER check_payer_application
BEFORE INSERT ON payer_applications
FOR EACH ROW EXECUTE PROCEDURE check_payer_application();
