-- Add down migration script here
DROP TABLE IF EXISTS problems;
DROP TRIGGER IF EXISTS refresh_contests_updated_at_step1 ON problems;
DROP TRIGGER IF EXISTS refresh_contests_updated_at_step2 ON problems;
DROP TRIGGER IF EXISTS refresh_contests_updated_at_step3 ON problems;

DROP TABLE IF EXISTS contests;
DROP TRIGGER IF EXISTS refresh_problems_updated_at_step1 ON contests;
DROP TRIGGER IF EXISTS refresh_problems_updated_at_step2 ON contests;
DROP TRIGGER IF EXISTS refresh_problems_updated_at_step3 ON contests;

DROP FUNCTION IF EXISTS refresh_updated_at_step1;
DROP FUNCTION IF EXISTS refresh_updated_at_step2;
DROP FUNCTION IF EXISTS refresh_updated_at_step3;