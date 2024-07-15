DROP TABLE IF EXISTS payments;
DROP FUNCTION IF EXISTS check_payment_data;
DROP FUNCTION IF EXISTS does_payer_have_pending_bills;

DROP TABLE IF EXISTS bills;
DROP TABLE IF EXISTS identities;
DROP TABLE IF EXISTS payers;
DROP TABLE IF EXISTS admins;

---------------------------------------------------
DROP FUNCTION IF EXISTS diesel_manage_updated_at(_tbl regclass);
DROP FUNCTION IF EXISTS diesel_set_updated_at();