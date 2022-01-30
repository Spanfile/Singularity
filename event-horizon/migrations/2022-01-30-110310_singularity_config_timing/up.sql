ALTER TABLE singularity_configs ADD timing TEXT NOT NULL DEFAULT "0 0 * * * ";
ALTER TABLE singularity_configs ADD last_run TEXT;
