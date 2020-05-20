-- added by 0.1.0-alpha1

CREATE SCHEMA IF NOT EXISTS steward;

CREATE TABLE IF NOT EXISTS steward.meta (
    at_migration INTEGER NOT NULL DEFAULT '0'
);

-- Insert default values if no row exists
INSERT INTO steward.meta
SELECT
WHERE NOT EXISTS (SELECT * FROM steward.meta);
