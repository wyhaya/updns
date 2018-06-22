

# updns

[![Build Status](https://img.shields.io/travis/wyhaya/updns.svg)](https://travis-ci.org/wyhaya/updns) [![Download](https://img.shields.io/npm/dt/updns.svg)](https://www.npmjs.com/package/updns) ![npm version](https://badge.fury.io/js/updns.svg) ![node](https://img.shields.io/node/v/updns.svg) [![License](https://img.shields.io/npm/l/updns.svg)](./LICENSE) [![codebeat](https://codebeat.co/badges/166a4edb-25e0-498b-9ac0-39e0b4daaffb)](https://codebeat.co/projects/github-com-wyhaya-updns-master)

---

updns is a DNS server developed using node, only need a simple configuration to get started, you can intercept any domain name and return any ip you need.

## Running as a Service

```
npm install -g updns
```

#### Config

We configure routing in the way of hosts

You can use `updns config` command and then call `vim` quick edit, or find the module's installation directory and edit the `config/hosts` file

The service can be bound to an IP and port, such as for a specific ethernet adapter or any valid loopback address (127.0.0.42)

You can specify standard domains, or utilize [regular expressions](https://www.regexpal.com "Regex Pal") for dynamic matching
```ini
bind              0.0.0.0:53     # address => port (requires restarting the service)
proxy             8.8.8.8        # proxy => DNS Server
google.com        1.1.1.1        # domain => IP
yahoo.com         2.2.2.2
/goo+gle\.com/    3.3.3.3        # regex: gooooooooooogle.com => IP
```
#### Start to use
```
updns start
```
You may use `sudo` to run this command because you will use the `53` port, make sure you have sufficient permissions.

Now change your local DNS server to `127.0.0.1` 🚀

#### Other

| Command          | Explain                                       |
| -------------    | -------------                                 |
| `updns start`    | Start the DNS service                         |
| `updns stop`     | Stop the DNS services                         |
| `updns config`   | Using `vim` to edit the configuration file    |
| `updns restart`  | Restart the dns service                       |
| `updns log`      | Using `less` to view log files                |
| `updns path`     | Display the installation directory of `updns` |
| `updns version`  | View version                                  |


## Create DNS Server
You can also create your DNS server as a module
```
npm install updns
```
If an IP address is not specified, the port will be bound globally (0.0.0.0)
```javascript
const updns = require('updns').createServer(53, '127.0.0.1')

updns.on('error', error => {
    console.log(error)
})

updns.on('listening', server => {
    console.log('DNS service has started')
})

updns.on('message', (domain, send, proxy) => {
    if(domain === 'google.com'){
        send('123.123.123.123')
    }else {
        proxy('8.8.8.8')
    }
})
```

## License
[MIT](./LICENSE) license
