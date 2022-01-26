CREATE TABLE evh_settings_type_values (
    id INTEGER NOT NULL,
    name TEXT UNIQUE NOT NULL,
    PRIMARY KEY (id)
);

INSERT INTO evh_settings_type_values (id, name) VALUES
    (0, "ActiveSingularityConfig");

CREATE TABLE evh_settings (
    id INTEGER NOT NULL,
    setting_type INTEGER UNIQUE NOT NULL,
    value TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY (setting_type) REFERENCES evh_settings_type_values (id)
);
