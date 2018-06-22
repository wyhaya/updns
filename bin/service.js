

const fs = require('fs')
const path = require('path')
const log = require('./log')
const hostsConfigPath = path.join(__dirname, './../config/hosts')


var bind = {
    address: null,
    port: 53
}
const proxy = []

const meta = new RegExp('\/(.*)\/')  // regex to find regex (the slashes are escaped on purpose because we're matching the regex markings themselves)
var hosts

function validateDomain (dom, ipaddr) {
    // google.com    8.8.8.8    # domain => ip
    // check for a valid hostname (per RFC 1034)
    // first character is a letter, middle is letters/digits/hyphens, last character is a letter or digit
    // each section must be 63 characters or less in length, with the entire domain no greater than 253
    // 253 bytes for the textual name, plus the trailing '.', and another byte to record the length = 255 max
    // https://tools.ietf.org/html/rfc1034#section-3.5
    let hostReg = /^((?:[a-z]+[a-z|0-9|-]{0,61}[a-z|0-9]\.)+[(a-z)]+)$/i

    return (dom.length <= 253 && hostReg.test(dom)) ? (ipaddr || dom) : false
}

function updateHosts () {
    hosts = fs
        .readFileSync(hostsConfigPath, 'utf-8')
        .split('\n')
        .map(host => {

            if(/^\s*#.+$/.test(host)) {
                return false
            }

            // bind    127.0.0.3:53    # listen-address : port
            let bindReg = /^\s*bind\s+((?:25[0-5]|2[0-4]\d|1\d\d|\d{1,2})(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|\d{1,2})){3})(?::(\d{1,5}))?\s*#?.*$/
            let binding = bindReg.exec(host)
            if(binding){
                bind.address = (binding[1] === '0.0.0.0') ? null : binding[1]
                bind.port = (binding[2] <= 65535) ? binding[2] : bind.port
                log.write(`Updns has bound to interface ${bind.address || '0.0.0.0(ANY/ALL)'} on port ${bind.port}`)
                return false
            }

            // proxy    8.8.8.8    # proxy => ip
            let proxyReg = /^\s*proxy\s+((\d{1,2}|1\d\d|2[0-4]\d|25[0-5])(\.(\d{1,2}|1\d\d|2[0-4]\d|25[0-5])){3})(\s*|\s+#.*)$/
            if(proxyReg.test(host)){
                proxy.push(host.match(proxyReg)[1])
                return false
            }

            let ipReg = /^((?:\d{1,2}|1\d\d|2[0-4]\d|25[0-5])(?:\.(?:\d{1,2}|1\d\d|2[0-4]\d|25[0-5])){3})$/ // matches a valid ip

            let rowParts = host.trim().replace(/\s\s+/g, ' ').split(' ')
            if (rowParts.length < 2) return false // must specify a domain and ip

            // /(.*\.)?goo+gle?\.(?:com|net|org)/    127.0.0.1    # dyanmic-domain (eg: gooooooooooooooogle.com) => ip
            let customRegEx = meta.exec(rowParts[0]) // allow the user to specify their own regex for dynamic matching
            let theHost = customRegEx ? new RegExp(customRegEx[1], 'i') : validateDomain(rowParts[0])

            let theIP = ipReg.exec(rowParts[1]) ? rowParts[1] : false // check for a valid IP address

            if(theHost && theIP) return {
                ip: theIP,
                domain: theHost
            }

            return false

        })
        .filter(Boolean)
}

updateHosts()
fs.watchFile(hostsConfigPath, updateHosts)

if(proxy.length === 0){

    process.send({
        state: 0,
        message: `
Please find this document: ${hostsConfigPath}
Add a correct proxy address: "proxy 8.8.8.8"`
    })

    process.exit(0)

}


const updns = require('./../lib').createServer(bind.port, bind.address)

updns.on('error', err => {
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


updns.on('message', (domain, send, proxyTo) => {
    let matchFound = false

    hosts.some(host => {
        if (domain.match(host.domain)) {
            matchFound = (typeof host.domain === 'object') ? validateDomain(domain, host.ip) : host.ip
            return true
        }
    })

    if(matchFound){
        send(matchFound)
    }else {
        proxyTo(proxy[0])
    }

    log.write(domain)

})


