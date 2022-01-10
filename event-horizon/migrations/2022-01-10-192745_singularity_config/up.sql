CREATE TABLE singularity_config (
    id INTEGER NOT NULL,
    dirty BOOLEAN NOT NULL,
    http_timeout INTEGER NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE singularity_adlist (
    id INTEGER NOT NULL,
    singularity_config_id INTEGER NOT NULL,
    source TEXT UNIQUE NOT NULL,
    format TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_config_id) REFERENCES singularity_config (id)
);

CREATE TABLE singularity_output (
    id INTEGER NOT NULL,
    singularity_config_id INTEGER NOT NULL,
    ty TEXT NOT NULL,
    destination TEXT UNIQUE NOT NULL,
    blackhole_address TEXT NOT NULL,
    deduplicate BOOLEAN NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_config_id) REFERENCES singularity_config (id)
);

CREATE TABLE singularity_whitelist (
    id INTEGER NOT NULL,
    singularity_config_id INTEGER NOT NULL,
    domain TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_config_id) REFERENCES singularity_config (id)
);

CREATE TABLE singularity_output_hosts_include (
    id INTEGER NOT NULL,
    singularity_output_id INTEGER NOT NULL,
    path TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_output_id) REFERENCES singularity_output(id)
);

CREATE TABLE singularity_output_pdns_lua (
    id INTEGER NOT NULL,
    singularity_output_id INTEGER UNIQUE NOT NULL,
    output_metric BOOLEAN NOT NULL,
    metric_name TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_output_id) REFERENCES singularity_output(id)
);
