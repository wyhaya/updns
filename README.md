# updns

> [!WARNING]  
> * !!! This project contains severe bugs
> * !!! No plans for fixes or patches
> * !!! DO NOT use it in any environment

---

updns is a simple DNS proxy server developed using `Rust`. You can intercept any domain name and return the ip you need

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
