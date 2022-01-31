# Event Horizon

## Development

Requirements for developing Event Horizon locally:

- Rust
- A container environment compatible with `docker-compose` (Docker, Podman, what have you)
- Optional: `npm` and a Sass compiler

To get started, run the local Redis and PDNS Recursor containers:

```
docker-compose -f docker-compose-dev.yml up -d
```

Create a file called `.env` in this directory with the following content. Modify it to your environment if needed.

```
EVH_LOG_LEVEL=debug
EVH_LISTEN_SOCKET=http
EVH_LISTEN_BIND=127.0.0.1:8053
DATABASE_URL=evh.sqlite
```

Create a file called `evh.toml` in this directory with the following content. Modify it to your environment if needed.

```
database_url = 'evh.sqlite'

[redis]
max_concurrent_imports = 5
max_error_lifetime = 300
max_import_lifetime = 300
max_stored_errors = 10
redis_url = 'redis://localhost'

[recursor]
hostname = ''
private_key_location = ''
remote_host_key = ''
username = ''
verify_remote_host_key = false
```

Run Event Horizon:

```
cargo run
```

If everything worked out, Event Horizon should start succesfully.

### SQLite

When starting, Event Horizon connects to an SQLite database in the file defined with the `database_url` setting in the `evh.toml` configuration file. During development, Diesel reads its location from the `.env` file from the `DATABASE_URL` setting.

### Redis

When starting, Event Horizon connects to a Redis database defined in the `redis_url` setting in the `evh.toml` configuration file. During development this is likely `redis://localhost` that runs in a local container as defined in `docker-compose-dev.yml`. It is possible to run Redis elsewhere if you don't have containers available.

### PDNS Recursor

TODO

### Sass

During normal development you don't have to touch Sass, NPM and the Bootstrap source files. The `static/` directory already contains a pre-compiled and minified Bootstrap CSS file ready for deployment. If you wish to edit the Bootstrap customisations in `stylesheet/`, however, you'll need `npm` and a Sass compiler such as `ruby-sass`. Then in order to compile and minify the `custom.scss` file, install the Bootstrap dependency locally:

```
npm i
```

Then navigate to the `stylesheet/` directory and compile the stylesheet:

```
sass custom.scss ../static/bootstrap.min.css --style compressed
```