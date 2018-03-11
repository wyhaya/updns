

const dgram = require('dgram')
const EventEmitter = require('events').EventEmitter
const DNSParse = require('./parse')


module.exports.createServer = function(port = 53) {

    let dnsServerEvent = new EventEmitter()
    this.server = dgram.createSocket('udp4')

    this.server.on('error',error => {

        dnsServerEvent.emit('error',error)
        return dnsServerEvent

    })

    this.server.on('listening',() => {

        dnsServerEvent.emit('listening', this.server)
        return dnsServerEvent
        
    })

    this.server.on('message',(message,rinfo) => {

        let query = DNSParse.parse(message)
        let domain = DNSParse.domainify(query.question.qname)
        
        let respond = (buf) => {
            this.server.send(buf, 0, buf.length, rinfo.port, rinfo.address)
        }
        
        dnsServerEvent.emit('message',domain, ip => {

            respond(DNSParse.response(query, 1, DNSParse.numify(ip)))
            
        }, proxy => {

            let proxySoket = dgram.createSocket('udp4')

            proxySoket.on('error',err => {
                dnsServerEvent.emit('error',err)
            })

            proxySoket.on('message', function(response) {
                respond(response)
                proxySoket.close()
            })

            proxySoket.send(message, 0, message.length, 53, proxy)

        })

        return dnsServerEvent

    })

    this.server.bind(port)
    
    return dnsServerEvent

}


