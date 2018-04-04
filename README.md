

# updns

[![Build Status](https://img.shields.io/travis/wyhaya/updns.svg)](https://travis-ci.org/wyhaya/updns) [![download](https://img.shields.io/npm/dt/updns.svg)](https://www.npmjs.com/package/updns) ![node](https://img.shields.io/node/v/updns.svg) [![license](https://img.shields.io/npm/l/updns.svg)](./LICENSE) [![codebeat](https://codebeat.co/badges/166a4edb-25e0-498b-9ac0-39e0b4daaffb)](https://codebeat.co/projects/github-com-wyhaya-updns-master) ![npm version](https://img.shields.io/npm/v/updns.svg)

---

updns is a DNS server developed using node, only need a simple configuration to get started, you can intercept any domain name and return any ip you need.

## Running as a Service

```
npm install -g updns
```

#### Config

We configure routing in the way of hosts

You need to find the installation path of the module first and configure it in `config/hosts`

```
proxy         8.8.8.8    # Proxy => DNS Server
google.com    1.1.1.1    # Domain => IP
yahoo.com     2.2.2.2
```
#### Start to use
```
updns start
```
You may use `sudo` to run this command because you will use the `53` port, make sure you have sufficient permissions.

Now change your local DNS server to `127.0.0.1` 🚀

#### Other

| Command          | Explain                                 |
| -------------    | -------------                           |
| `updns start`    | Start the DNS service                   |
| `updns stop`     | Stop the DNS services                   |
| `updns config`   | Using VI to edit the configuration file |
| `updns reload`   | Reload the hosts configuration file     |
| `updns log`      | Using less to view log files            |
| `updns -v`       | View version                            |


## Create DNS Server
You can also create your DNS server as a module
```
npm install updns
```
```javascript
const updns = require('updns').createServer(53)

updns.on('error', err => {
    console.log('There is a mistake')
})

updns.on('listening', server => {
    console.log('The DNS server has been started')
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
