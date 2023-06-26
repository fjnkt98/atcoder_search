CREATE TABLE IF NOT EXISTS "submissions" (
    "id" BIGINT PRIMARY KEY,
    "epoch_second" BIGINT NOT NULL,
    "problem_id" TEXT NOT NULL,
    "contest_id" TEXT,
    "user_id" TEXT,
    "language" TEXT,
    "point" DOUBLE PRECISION,
    "length" INTEGER,
    "result" TEXT,
    "execution_time" INTEGER
);

CREATE INDEX "submissions_epoch_second_index" ON "submissions" ("epoch_second");
CREATE INDEX "submissions_problem_id_index" ON "submissions" ("problem_id");
CREATE INDEX "submissions_contest_id_index" ON "submissions" ("contest_id");
CREATE INDEX "submissions_user_id_index" ON "submissions" ("user_id");
