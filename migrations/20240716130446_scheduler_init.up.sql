CREATE TYPE job_priority AS ENUM (
    'low', 'medium', 'high'
);

CREATE TYPE job_status AS ENUM (
    'failed', 'running', 'success', 'queued'
);

CREATE OR REPLACE FUNCTION get_job_priority_level ("value" JOB_PRIORITY)
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

CREATE TABLE jobs (
    "id" UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    "created_at" TIMESTAMP WITHOUT TIME ZONE
        NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "name" VARCHAR(50) NOT NULL,
    "updated_at" TIMESTAMP,

    "data" JSON NOT NULL,
    "deadline" TIMESTAMP NOT NULL,
    "failed_attempts" INTEGER NOT NULL DEFAULT 0,
    "last_retry" TIMESTAMP,
    "priority" JOB_PRIORITY NOT NULL DEFAULT 'medium',
    "status" JOB_STATUS NOT NULL DEFAULT 'queued'
);
SELECT manage_updated_at('jobs');

CREATE OR REPLACE FUNCTION check_job_data()
    RETURNS TRIGGER
    AS $$
BEGIN
    -- data must not be null
    IF (NEW.data IS NULL) THEN
        RETURN NEW.data;
    END IF;

    IF (json_typeof(NEW.data::json) != 'object') THEN
        RAISE EXCEPTION '"data" column is not a JSON object';
    END IF;

    IF (NEW.data->>'type' IS NULL) THEN
        RAISE EXCEPTION '"data.type" is missing';
    END IF;

    RETURN NEW;
END
$$
LANGUAGE plpgsql;

CREATE TRIGGER check_job_data
BEFORE INSERT OR UPDATE ON jobs
FOR EACH ROW EXECUTE PROCEDURE check_job_data();
