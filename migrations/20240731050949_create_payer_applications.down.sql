DROP TABLE IF EXISTS payer_applications;

DROP TRIGGER check_identity_if_not_occupied_from_payers_application ON identities;
DROP FUNCTION IF EXISTS check_payer_application;
