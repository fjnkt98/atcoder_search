-- Add up migration script here
CREATE FUNCTION refresh_updated_at_step1() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.updated_at = OLD.updated_at THEN NEW.updated_at := NULL;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION refresh_updated_at_step2() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.updated_at IS NULL THEN NEW.updated_at := OLD.updated_at;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE FUNCTION refresh_updated_at_step3() RETURNS TRIGGER AS
$$
BEGIN
IF NEW.updated_at IS NULL THEN NEW.updated_at := CURRENT_TIMESTAMP;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE IF NOT EXISTS contests (
    contest_id TEXT PRIMARY KEY,
    start_epoch_second BIGINT NOT NULL,
    duration_second BIGINT NOT NULL,
    title TEXT NOT NULL,
    rate_change TEXT NOT NULL,
    category TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TRIGGER refresh_contests_updated_at_step1 BEFORE
UPDATE ON contests FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step1();

CREATE TRIGGER refresh_contests_updated_at_step2 BEFORE
UPDATE OF updated_at ON contests FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step2();

CREATE TRIGGER refresh_contests_updated_at_step3 BEFORE
UPDATE ON contests FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step3();

CREATE TABLE IF NOT EXISTS problems (
    problem_id TEXT PRIMARY KEY,
    contest_id TEXT NOT NULL REFERENCES contests (contest_id) ON DELETE CASCADE,
    problem_index TEXT NOT NULL,
    name TEXT NOT NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    html TEXT NOT NULL,
    difficulty INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX contest_id_index ON problems (contest_id);

CREATE TRIGGER refresh_problems_updated_at_step1 BEFORE
UPDATE ON problems FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step1();

CREATE TRIGGER refresh_problems_updated_at_step2 BEFORE
UPDATE OF updated_at ON problems FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step2();

CREATE TRIGGER refresh_problems_updated_at_step3 BEFORE
UPDATE ON problems FOR EACH ROW EXECUTE PROCEDURE refresh_updated_at_step3();