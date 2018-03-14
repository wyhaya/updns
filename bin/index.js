

const path = require('path')
const EventEmitter = require('events').EventEmitter
const hostsConfigPath = path.join(__dirname, './../config/hosts')


const proxy = []
const hosts
    =
    require('fs')
        .readFileSync(hostsConfigPath, 'utf-8')
        .split('\n')
        .map(host => {

            if(/^\s*#.+$/.test(host)) {
                return false
            }

            // proxy    8.8.8.8    # proxy => ip
            let proxyReg = /^\s*proxy\s+((\d{1,2}|1\d\d|2[0-4]\d|25[0-5])(\.(\d{1,2}|1\d\d|2[0-4]\d|25[0-5])){3})(\s*|\s+#.*)$/
            if(proxyReg.test(host)){
                proxy.push(host.match(proxyReg)[1])
                return false
            }

            // google.com    8.8.8.8    # domain => ip
            let hostReg = /^\s*([(\w|\-)]+(\.[a-z]+)+)\s+((\d{1,2}|1\d\d|2[0-4]\d|25[0-5])(\.(\d{1,2}|1\d\d|2[0-4]\d|25[0-5])){3})(\s*|\s+#.*)+$/
            if(hostReg.test(host)) {
                return {
                    ip: host.match(hostReg)[3],
                    domain: host.match(hostReg)[1]
                }
            }

            return false

        })
        .filter(host => !!host)


if(proxy.length === 0){

    process.send({
        state: 0,
        message: `
Please find this document: ${hostsConfigPath}
Add a correct proxy address: "proxy 8.8.8.8"`
    })

    process.exit(0)

}


const updns = require('./../lib').createServer(53)
const domainEvent = new EventEmitter()

hosts.forEach(host => {
    domainEvent.on(host.domain, send => {
        send(host.ip)
    })
})


updns.on('error',err => {
    process.send({
        state: 0, 
        message: err.toString()
    })
})

updns.on('listening', () => {
    process.send({
        state: 1,
        message: 'Service has been started'
    })
})


updns.on('message',(domain, send, proxyTo) => {
    if(domainEvent.listenerCount(domain)) {
        domainEvent.emit(domain,send)
    }else {
        proxyTo(proxy[0])
    }
})

