-- integer primary keys do not need to be marked NOT NULL
-- REFERENCES on id should use ON UPDATE CASCADE
CREATE TABLE problem (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    file_hash TEXT NOT NULL UNIQUE
) STRICT;
-- a problem instance
CREATE TABLE instance (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    problem INTEGER NOT NULL REFERENCES problem ON UPDATE CASCADE,
    seed INTEGER NOT NULL,
    UNIQUE (problem, seed)
) STRICT;
-- a wasm solution
CREATE TABLE solution (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    file_hash INTEGER NOT NULL UNIQUE
) STRICT;
-- a user of the server
CREATE TABLE user (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    github_id INTEGER NOT NULL UNIQUE
) STRICT;
-- a solution applied to a problem is a submission. it is associated with a user.
CREATE TABLE submission (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    problem INTEGER NOT NULL REFERENCES problem ON UPDATE CASCADE,
    solution INTEGER NOT NULL REFERENCES solution ON UPDATE CASCADE,
    user INTEGER NOT NULL REFERENCES user ON UPDATE CASCADE,
    UNIQUE (problem, solution, user)
) STRICT;
-- a solution applied to a problem instance results in an execution
CREATE TABLE execution (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    fuel_used INTEGER NOT NULL,
    instance INTEGER NOT NULL REFERENCES instance ON UPDATE CASCADE,
    solution INTEGER NOT NULL REFERENCES solution ON UPDATE CASCADE,
    UNIQUE (instance, solution)
) STRICT;