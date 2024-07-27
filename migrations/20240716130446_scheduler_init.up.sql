CREATE TYPE task_priority AS ENUM (
    'low', 'medium', 'high'
);

CREATE TYPE task_status AS ENUM (
    'failed', 'running', 'success', 'queued'
);

CREATE OR REPLACE FUNCTION get_task_priority_level ("value" TASK_PRIORITY)
    RETURNS INTEGER
    AS $$
BEGIN
    IF ("value" = 'high') THEN
        RETURN 3;
    ELSIF ("value" = 'medium') THEN
        RETURN 2;
    END IF;
    RETURN 1;
END;
$$
LANGUAGE plpgsql;

CREATE TABLE tasks (
    "id" UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    "created_at" TIMESTAMP WITHOUT TIME ZONE
        NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "updated_at" TIMESTAMP,

    "data" JSONB NOT NULL,
    "deadline" TIMESTAMP NOT NULL,
    "failed_attempts" INTEGER NOT NULL DEFAULT 0,
    "last_retry" TIMESTAMP,
    "periodic" BOOLEAN NOT NULL DEFAULT false,
    "priority" TASK_PRIORITY NOT NULL DEFAULT 'medium',
    "status" TASK_STATUS NOT NULL DEFAULT 'queued'
);

CREATE OR REPLACE FUNCTION check_task_data()
    RETURNS TRIGGER
    AS $$
BEGIN
    -- data must not be null
    IF (NEW.data IS NULL) THEN
        RETURN NEW.data;
    END IF;

    IF (jsonb_typeof(NEW.data::jsonb) != 'object') THEN
        RAISE EXCEPTION '"data" column is not a JSON object';
    END IF;

    IF (NEW.data->>'type' IS NULL) THEN
        RAISE EXCEPTION '"data.type" is missing';
    END IF;

    RETURN NEW;
END
$$
LANGUAGE plpgsql;

CREATE TRIGGER check_task_data
BEFORE INSERT OR UPDATE ON tasks
FOR EACH ROW EXECUTE PROCEDURE check_task_data();
