# Event Horizon

## Development

Requirements for developing Event Horizon locally:

- Rust, preferably the latest nightly or at least the latest stable
- A container environment compatible with `docker-compose` (Docker, Podman, what have you). I use rootless Podman personally, so all the development containers are designed to run in a rootless container environment
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
control_socket = 'pdns-recursor/pdns_recursor.controlsocket'
```

Run Event Horizon:

```
cargo run
```

If everything worked out, Event Horizon should start succesfully. Below are some additional notes and guides on the various components Event Horizon relies on.

### SQLite

When starting, Event Horizon connects to an SQLite database in the file defined with the `database_url` setting in the `evh.toml` configuration file. If the file doesn't exist, it will be created. The database migrations in [`./migrations`](migrattions/) are embedded to the Event Horizon binary during build-time and Event Horizon will run them automatically if required. During development, Diesel reads its location from the `.env` file from the `DATABASE_URL` setting.

### Redis

When starting, Event Horizon connects to a Redis database defined in the `redis_url` setting in the `evh.toml` configuration file. During development this is `redis://localhost`, since Redis runs in a local container as defined in `docker-compose-dev.yml` and has its port exposed to the local host.

### PDNS Recursor

The PDNS Recursor container is configured to have the local directory `./pdns-recursor` mounted to `/var/run/pdns-recursor` inside the container. This allows the Recursor control socket `pdns-recursor.controlsocket` be exposed to the host system during development so Event Horizon is able to communicate with Recursor, and you're able to communicate with it as well for debugging.

The file [`recursor-dev.conf`](recursor-dev.conf) is mounted to Recursor's configuration directory `/etc/powerdns/recursor.d`, so you can edit Recursor's configuration with this file if needed.

The Recursor container is the same as PowerDNS' Recursor container [`pdns-recursor-master`](https://hub.docker.com/r/powerdns/pdns-recursor-master), except it uses the root user inside the container (see [containers/Dockerfile-pdns-recursor](containers/Dockerfile-pdns-recursor)). This is ideal in rootless container environments where the local user's ID is mapped to the root user in the container. Normally the container would use its own `pdns` user with the ID 953, which would require additional configuration to cleanly map to your local user ID.

### Bootstrap and Sass

During normal development you don't have to touch Sass, NPM and the Bootstrap source files. The `static/` directory already contains a pre-compiled and minified Bootstrap CSS file ready for deployment. If you wish to edit the Bootstrap customisations in `stylesheet/`, however, you'll need `npm` and a Sass compiler such as `ruby-sass`. Then in order to compile and minify the `custom.scss` file, install the Bootstrap and Bootswatch dependencies locally:

```
npm i
```

Then navigate to the `stylesheet/` directory and compile the stylesheet:

```
sass custom.scss ../static/bootstrap.min.css --style compressed
```