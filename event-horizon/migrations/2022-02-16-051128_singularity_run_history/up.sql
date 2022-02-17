CREATE TABLE singularity_run_history_results (
    id INTEGER NOT NULL,
    name TEXT UNIQUE NOT NULL,
    PRIMARY KEY (id)
);

INSERT INTO singularity_run_history_results (id, name) VALUES
    (0, "Success"),
    (1, "SuccessWithErrors"),
    (2, "SuccessWithWarnings"),
    (3, "Failed");

CREATE TABLE singularity_run_histories (
    run_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    result INTEGER NOT NULL,
    PRIMARY KEY (run_id),
    FOREIGN KEY (result) REFERENCES singularity_run_history_results (id)
);
