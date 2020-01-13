

# updns

[![Build Status](https://img.shields.io/travis/wyhaya/updns.svg?style=flat-square)](https://travis-ci.org/wyhaya/updns)
[![Crates.io](https://img.shields.io/crates/v/updns.svg?style=flat-square)](https://crates.io/crates/updns)
[![Crates.io](https://img.shields.io/crates/l/updns.svg?style=flat-square)](https://github.com/wyhaya/updns/blob/master/LICENSE)

---

updns is a simple DNS proxy server developed using `Rust`. You can intercept any domain name and return the ip you need

## Install

[Download](https://github.com/wyhaya/updns/releases) the binary from the release page

Or use `cargo` to install

```bash
cargo install updns
```

## Start to use 🚀

```bash
updns
# or
updns -c /your/hosts
```

You may use `sudo` to run this command because you will use the `53` port

## Running in docker

Build docker image
```bash
docker build -t updns .
```

Start up
```bash
docker run -d --name updns -p 53:53/udp -v /root/updns/:/root/.updns/ --restart always updns
```

## Command

```
Usage:
    updns [COMMAND] [OPTION]
            
Command:
    add        Add a DNS record
    ls         Print all configured DNS records
    config     Call 'vim' to edit the configuration file
    path       Print related directories
    help       Print help information
    version    Print version information

Option:
    -c    Specify a config file
    -i    Check the interval time of the configuration file
          format: 1ms, 1s, 1m, 1h, 1d
```

## Config

You can use `updns config` command and then call `vim` edit, or find `~/.updns/config`  edit

You can specify standard domains, or utilize [regular expressions](https://rustexp.lpil.uk "rustexp") for dynamic matching

> Regular expression starts with `~`

```ini
bind     0.0.0.0:53      # Binding address
proxy    8.8.8.8:53      # Proxy address
timeout  2s              # Proxy timeout (format: 1ms, 1s, 1m, 1h, 1d)

# Domain matching
example.com              1.1.1.1
*.example.com            2.2.2.2
~^\w+\.example\.[a-z]+$  3.3.3.3

# IPv6
test.com                ::

# Import from other file
import /other/hosts
```

## Reference

[Building a DNS server in Rust](https://github.com/EmilHernvall/dnsguide)

## License

[MIT](./LICENSE) license
