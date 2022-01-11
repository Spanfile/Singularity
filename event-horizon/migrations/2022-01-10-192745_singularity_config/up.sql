CREATE TABLE singularity_configs (
    id INTEGER NOT NULL,
    dirty BOOLEAN NOT NULL,
    http_timeout INTEGER NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE singularity_adlists (
    id INTEGER NOT NULL,
    singularity_config_id INTEGER NOT NULL,
    source TEXT UNIQUE NOT NULL,
    format TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_config_id) REFERENCES singularity_configs (id) ON DELETE CASCADE,
    UNIQUE (singularity_config_id, source),
    CHECK (format IN ("Hosts", "Domains", "Dnsmasq"))
);

CREATE TABLE singularity_outputs (
    id INTEGER NOT NULL,
    singularity_config_id INTEGER NOT NULL,
    ty TEXT NOT NULL,
    destination BLOB UNIQUE NOT NULL,
    blackhole_address TEXT NOT NULL,
    deduplicate BOOLEAN NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_config_id) REFERENCES singularity_configs (id) ON DELETE CASCADE,
    UNIQUE (singularity_config_id, destination),
    CHECK (ty IN ("Hosts", "PdnsLua"))
);

CREATE TABLE singularity_whitelists (
    id INTEGER NOT NULL,
    singularity_config_id INTEGER NOT NULL,
    domain TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_config_id) REFERENCES singularity_configs (id) ON DELETE CASCADE,
    UNIQUE (singularity_config_id, domain)
);

CREATE TABLE singularity_output_hosts_includes (
    id INTEGER NOT NULL,
    singularity_output_id INTEGER NOT NULL,
    path BLOB NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_output_id) REFERENCES singularity_outputs(id) ON DELETE CASCADE,
    UNIQUE (singularity_output_id, path)
);

CREATE TABLE singularity_output_pdns_lua (
    id INTEGER NOT NULL,
    singularity_output_id INTEGER UNIQUE NOT NULL,
    output_metric BOOLEAN NOT NULL,
    metric_name TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (singularity_output_id) REFERENCES singularity_outputs(id) ON DELETE CASCADE
);
