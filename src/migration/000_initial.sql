CREATE TABLE file (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    file_hash INTEGER NOT NULL UNIQUE,
    file_size INTEGER NOT NULL
) STRICT;

-- a problem benchmark instance
CREATE TABLE instance (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    problem INTEGER NOT NULL REFERENCES file,
    seed INTEGER NOT NULL,
    UNIQUE (problem, seed)
) STRICT;

-- a wasm solution
CREATE TABLE solution (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    program INTEGER NOT NULL REFERENCES file,
    problem INTEGER NOT NULL REFERENCES file,
    -- how many random tests did this solution pass
    random_tests INTEGER NOT NULL,
    -- program can only be submitted to a problem once
    UNIQUE (program, problem)
) STRICT;

-- a random test "or benchmark test" failed
CREATE TABLE failure (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    solution INTEGER NOT NULL UNIQUE REFERENCES solution,
    seed INTEGER NOT NULL,
    message TEXT NOT NULL
) STRICT;

-- a user of the server
CREATE TABLE user (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    github_id INTEGER NOT NULL UNIQUE,
    github_login TEXT NOT NULL
) STRICT;

-- who uploaded the solution
CREATE TABLE submission (
    id INTEGER PRIMARY KEY,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch('now')),
    solution INTEGER NOT NULL REFERENCES file,
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