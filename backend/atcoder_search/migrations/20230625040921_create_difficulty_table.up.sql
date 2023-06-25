CREATE TABLE IF NOT EXISTS "difficulties" (
    "problem_id" TEXT PRIMARY KEY,
    "slope" DOUBLE PRECISION,
    "intercept" DOUBLE PRECISION,
    "variance" DOUBLE PRECISION,
    "difficulty" INTEGER,
    "discrimination" DOUBLE PRECISION,
    "irt_loglikelihood" DOUBLE PRECISION,
    "irt_users" DOUBLE PRECISION,
    "is_experimental" BOOLEAN,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER refresh_difficulties_updated_at_step1 BEFORE
UPDATE ON difficulties FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step1();

CREATE TRIGGER refresh_difficulties_updated_at_step2 BEFORE
UPDATE OF updated_at ON difficulties FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step2();

CREATE TRIGGER refresh_difficulties_updated_at_step3 BEFORE
UPDATE ON difficulties FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step3();
