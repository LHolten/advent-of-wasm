-- integer primary keys do not need to be marked NOT NULL
-- REFERENCES on id should use 
CREATE TABLE problem (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    file_hash INTEGER NOT NULL UNIQUE
) STRICT;

-- a problem benchmark instance
CREATE TABLE instance (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    problem INTEGER NOT NULL REFERENCES problem,
    seed INTEGER NOT NULL,
    UNIQUE (problem, seed)
) STRICT;

-- a wasm solution
CREATE TABLE solution (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    file_hash INTEGER NOT NULL,
    problem INTEGER NOT NULL REFERENCES problem,
    -- how many random tests did this solution pass
    random_tests INTEGER NOT NULL,
    -- solutions can only be submitted to a problem once
    UNIQUE (file_hash, problem)
) STRICT;

-- a random test "or benchmark test" failed
CREATE TABLE failure (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    solution INTEGER NOT NULL UNIQUE REFERENCES solution,
    seed INTEGER NOT NULL
) STRICT;

-- a user of the server
CREATE TABLE user (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    github_id INTEGER NOT NULL UNIQUE
) STRICT;

-- who uploaded the solution
CREATE TABLE submission (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    solution INTEGER NOT NULL REFERENCES solution,
    user INTEGER NOT NULL REFERENCES user,
    UNIQUE (solution, user)
) STRICT;

-- a solution applied to a problem instance results in an execution
CREATE TABLE execution (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    fuel_used INTEGER NOT NULL,
    -- answer can be null if the solution crashed
    answer INTEGER,
    instance INTEGER NOT NULL REFERENCES instance,
    solution INTEGER NOT NULL REFERENCES solution,
    UNIQUE (instance, solution)
) STRICT;