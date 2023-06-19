CREATE TABLE IF NOT EXISTS "users" (
    "user_name" TEXT PRIMARY KEY,
    "rating" INTEGER NOT NULL,
    "highest_rating" INTEGER NOT NULL,
    "affiliation" TEXT,
    "birth_year" INTEGER,
    "country" TEXT,
    "crown" TEXT,
    "join_count" INTEGER NOT NULL,
    "rank" INTEGER NOT NULL,
    "wins" INTEGER NOT NULL,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER refresh_users_updated_at_step1 BEFORE
UPDATE ON users FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step1();

CREATE TRIGGER refresh_users_updated_at_step2 BEFORE
UPDATE OF updated_at ON users FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step2();

CREATE TRIGGER refresh_users_updated_at_step3 BEFORE
UPDATE ON users FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step3();