-- From Diesel
CREATE OR REPLACE FUNCTION manage_updated_at(_tbl regclass) RETURNS VOID AS $$
BEGIN
    EXECUTE format('CREATE TRIGGER set_updated_at BEFORE UPDATE ON %s
                    FOR EACH ROW EXECUTE PROCEDURE set_updated_at()', _tbl);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger AS $$
BEGIN
    IF (
        NEW IS DISTINCT FROM OLD AND
        NEW.updated_at IS NOT DISTINCT FROM OLD.updated_at
    ) THEN
        NEW.updated_at := current_timestamp;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

----------------------------------------------------------------------------------------
CREATE TABLE admins (
    "id" BIGINT PRIMARY KEY NOT NULL,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    -- sometimes administrators' user data cannot load properly from
    -- Discord so we need to keep as it is until we can get their name.
    "name" VARCHAR(50),
    "updated_at" TIMESTAMP,

    CONSTRAINT non_empty_name CHECK(length("name") > 0)
);
SELECT manage_updated_at('admins');

CREATE TABLE payers (
    "id" BIGINT PRIMARY KEY NOT NULL,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "name" VARCHAR(50) NOT NULL,
    "updated_at" TIMESTAMP,

    CONSTRAINT non_empty_name CHECK(length("name") > 0)
);
SELECT manage_updated_at('payers');

-- stores either UUID or username data (both Bedrock and Java)
CREATE TABLE identities (
    "id" BIGINT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    "payer_id" BIGINT NOT NULL REFERENCES payers(id),
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "name" VARCHAR(100) UNIQUE,
    "uuid" UUID UNIQUE,

    -- You cannot have any combinations like this...
    UNIQUE ("payer_id", "uuid"),
    UNIQUE ("payer_id", "name"),

    CONSTRAINT valid_name CHECK (length("name") > 2),
    CONSTRAINT check_if_either_info_exists CHECK ("uuid" IS NOT NULL OR "name" IS NOT NULL)
);

CREATE TABLE bills (
    "id" BIGINT PRIMARY KEY NOT NULL GENERATED ALWAYS AS IDENTITY,
    "created_at" TIMESTAMP WITHOUT TIME ZONE NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    
    "created_by" BIGINT NOT NULL, -- creator's snowflake ID
    "updated_at" TIMESTAMP,

    -- TODO: Make an explicit currency type
    "currency" VARCHAR(3) NOT NULL,
    "deadline" DATE NOT NULL,
    "price" NUMERIC(2) NOT NULL,

    CONSTRAINT valid_currency CHECK (length("currency") = 3),
    CONSTRAINT valid_price CHECK (price > 0)
);

SELECT manage_updated_at('bills');

CREATE TABLE payments (
    "id" UUID PRIMARY KEY NOT NULL DEFAULT gen_random_uuid(),
    "created_at" TIMESTAMP WITHOUT TIME ZONE
        NOT NULL
        DEFAULT (now() at TIME ZONE ('utc')),
    "updated_at" TIMESTAMP,

    "payer_id" BIGINT NOT NULL REFERENCES payers(id),
    "bill_id" BIGINT NOT NULL REFERENCES bills(id),
    -- { "status": { "type": "success" | "failed" | "pending" | "" }, ... }
    "data" JSON NOT NULL,

    UNIQUE ("payer_id", "bill_id")
);

CREATE OR REPLACE FUNCTION does_payer_have_pending_bills (payer_id BIGINT)
    RETURNS BOOLEAN
    AS $$
DECLARE
    "successful_payments" INTEGER;
    "total_bills" INTEGER;
BEGIN
    SELECT COUNT(*) INTO "total_bills" FROM bills;

    SELECT COUNT(*) INTO "successful_payments" FROM bills bill
    INNER JOIN payments payment ON payment.bill_id = bill.id
    WHERE payment.data->'status'->>'type' = 'success';

    ASSERT "successful_payments" <= "total_bills", 'Unexpected successful_payments > total_bills, payer_id = %', payer_id;
    RETURN "successful_payments" != "total_bills";
END;
$$
LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION check_payment_data ()
    RETURNS TRIGGER
    AS $$
DECLARE
    "status_type" TEXT;
BEGIN
    -- PostgreSQL runs this trigger first before they validate
    -- perform all table constraints.
    IF (NEW.data IS NULL) THEN
        RETURN NEW;
    END IF;

    IF (NEW.payer_id IS NULL) THEN
        RETURN NEW;
    END IF;

    IF (json_typeof(NEW.data::json) != 'object') THEN
        RAISE EXCEPTION '"data" column is not a JSON object';
    END IF;

    IF (NEW.data->'status'->>'type' IS NULL) THEN
        RAISE EXCEPTION '"data.status.type" is missing';
    END IF;
    SELECT NEW.data->'status'->>'type' INTO "status_type";

    IF (
        "status_type" <> 'success'
        AND "status_type" <> 'failed'
        AND "status_type" <> 'pending'
    ) THEN
        RAISE EXCEPTION 'invalid status in "data.status.type", value = %', "status_type";
    END IF;

    IF (TG_OP = 'INSERT') THEN
        -- Make sure we don't have success or failed status upon inserting
        -- payment data. Guild administrators must validate payer's payment first.
        IF ("status_type" <> 'pending') THEN
            RAISE EXCEPTION '% status is not allowed for insert operations in "data.status.type"', "status_type";
        END IF;
    END IF;

    IF (OLD IS NOT NULL) THEN
        NEW.updated_at := current_timestamp;
    END IF;

    RETURN NEW;
END;
$$
LANGUAGE plpgsql;

CREATE TRIGGER check_payment_data
BEFORE INSERT OR UPDATE ON payments
FOR EACH ROW EXECUTE PROCEDURE check_payment_data();
