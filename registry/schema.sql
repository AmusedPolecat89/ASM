CREATE TABLE IF NOT EXISTS runs (
    date TEXT NOT NULL,
    "commit" TEXT NOT NULL,
    plan_name TEXT NOT NULL,
    plan_hash TEXT NOT NULL,
    job_id INTEGER NOT NULL,
    params TEXT NOT NULL,
    metrics TEXT NOT NULL
);
