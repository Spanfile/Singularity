# Singularity

CLI tool for pulling known malicious domains into a blackhole list. Primarily meant to be used with PDNS Recursor. The tool can pull in blackholed domains from multiple adlist sources and output them into multiple places in various formats.

## Usage

* Basic usage: `singularity`

### CLI options

All CLI options can be seen with the `--help` flag. The options are:

* `-v`, `--verbose`: enable additional debug output to stdout
* `-c`, `--config`: a custom configuration file to use instead of the default
* `-t`, `--timeout`: timeout in milliseconds to wait for each HTTP request to succeed (default: 1000)

### Configuration file

By default, the tool will use a confiuration file in the current system-dependent location. On Linux, this is `$HOME/.config/singularity/singularity.conf`. The file will be created if it doesn't exist and will contain empty values.

Complete example configuration file:

```toml
[[adlist]]
source = "https://raw.githubusercontent.com/StevenBlack/hosts/master/hosts"
format = "hosts"

[[adlist]]
source = "file:/absolute/path"
format = "domains"

[[output]]
type = "hosts"
destination = "/etc/powerdns/hosts"
blackhole-address = "0.0.0.0"
include = ["extra-hosts"]

[[output]]
type = "pdns-lua"
destination = "/etc/powerdns/blackhole.lua"
blackhole-address = "::"

```

#### `adlist`

An array of objects describing adlist sources. They have two keys:
* `source`: URL to the source of the adlist. The URL scheme can be `http`, `https` or `file`. If it's a `file` URL, its path will be interpreted as an absolute filesystem path in the local system.
* `format`: the format the adlist's entries are in. This option can be omitted for the default `hosts` value. The value can be either:
    * `hosts`: standard `/etc/hosts`-style entries; `0.0.0.0 malicious.domain`. It is assumed the address in each entry is the unspecfied `0.0.0.0` IP address. Entries that have a different IP address or have an IP address as the domain are ignored.
    * `domains`: each line is just a domain name: `malicious.domain`.

Regardless of the source or format, any lines in an adlist beginning with a `#` are ignored and will not be included in the output.

### `output`

An array of objects describing where and how to output the blackholed domains. The type of each output is specified with the `type` key. The possible types are:
* `hosts`: output a standard hosts-format where each line is in the format of `<blackhole-address> <name>`. Other hosts-files can be included in the output by settings their paths in the `include` array option.
* `pdns-lua`: output a Lua script that can be used with the `lua-dns-script` configuration option in PDNS Recursor. The script will have each blackholed domain hardcoded into it. By using the `preresolve()` function, the script will respond to queries for the blackholed domains with either an `A`-record or an `AAAA`-record containing the `blackhole-address`. The type of the record depends on whether the `blackhole-address` is an IPv4- or an IPv6-address.

In all output types, the default `blackhole-address` is `0.0.0.0` and can be changed per-output.
