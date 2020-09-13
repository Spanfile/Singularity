# Singularity

CLI tool for pulling known malicious domains into a blackhole list. Primarily meant to be used with PDNS Recursor. The output of the final blackhole list is in the standard hosts-format.

## Usage

### CLI options

All CLI options can be seen with the `--help` flag. The options are:

* `-v`, `--verbose`: enable additional debug output to stdout
* `-c`, `--config`: a custom configuration file to use instead of the default
* `-t`, `--timeout`: timeout in milliseconds to wait for each HTTP request to succeed (default: 1000)

### Configuration file

By default, the tool will use a confiuration file in the current system-dependent location. On Linux, this is `$HOME/.config/singularity/singularity.conf`. The file will be created if it doesn't exist and filled as such:

```toml
adlists = [
  {source = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts", format = "hosts"},
]
blackhole-address = "0.0.0.0"
output = "/etc/powerdns/hosts"
include = []
```

#### `adlists`

An array of objects describing adlist sources. They have two keys:
* `source`: URL to the source of the adlist. The URL scheme can be `http`, `https` or `path`. If it's a `path` URL, its path will be interpreted as an absolute path.
* `format`: the format the adlist's entries are in. It can be either:
    * `hosts`: standard `/etc/hosts`-style entries; `0.0.0.0 malicious.domain`. It is assumed the address in the entry is the unspecfied `0.0.0.0` address. Entries that have a different address or have an address for the domain are ignored.
    * `domains`: each line is just a domain name: `malicious.domain`.

Regardless of the source or format, any lines in an adlist beginning with a `#` are ignored and won't be included in the output.

#### `blackhole-address`

The blackhole address used in the output hosts-file.

#### `output`

Where to write the final hosts-file.

#### `include`

Array of paths to additional hosts-files to include in the output.

## PDNS Recursor configuration

The included `blackhole.conf` file corresponds to the tool's default configuration and can be placed in `/etc/powerdns/recursor.d` to be included in the Recursor's configuration. If the Recursor is already configured to include some hosts-file, the configuration entries for it should be removed and the tool should be configured to include the wanted hosts-files (the `include`-configuration option).
