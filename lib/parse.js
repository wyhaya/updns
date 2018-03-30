

// These methods are transformed from dnsjack
// GitHub: https://github.com/mafintosh/dnsjack


const bitSlice = (b, offset, length) => {

    return (b >>> (7 - (offset + length - 1))) & ~ (0xff << length)

}


module.exports.parse = buf => {
    let header = {}
    let question = {}
    let b = buf.slice(2,3).toString('binary', 0, 1).charCodeAt(0)

    header.id = buf.slice(0,2)
    header.qr = bitSlice(b,0,1)
    header.opcode = bitSlice(b,1,4)
    header.aa = bitSlice(b,5,1)
    header.tc = bitSlice(b,6,1)
    header.rd = bitSlice(b,7,1)

    b = buf.slice(3,4).toString('binary', 0, 1).charCodeAt(0)

    header.ra = bitSlice(b,0,1)
    header.z = bitSlice(b,1,3)
    header.rcode = bitSlice(b,4,4)

    header.qdcount = buf.slice(4,6)
    header.ancount = buf.slice(6,8)
    header.nscount = buf.slice(8,10)
    header.arcount = buf.slice(10, 12)

    question.qname = buf.slice(12, buf.length-4)
    question.qtype = buf.slice(buf.length-4, buf.length-2)
    question.qclass = buf.slice(buf.length-2, buf.length)

    return {
        header:header, 
        question:question
    }
}


module.exports.domainify = qnameBuf => {

    let parts = []

    for (let i = 0; i < qnameBuf.length && qnameBuf[i];) {
        let [length, offset] = [qnameBuf[i], i + 1]
        parts.push(
            qnameBuf.slice(offset, offset + length).toString()
        )
        i = offset + length
    }

    return parts.join('.')

}


module.exports.numify = ip => {

    ip = ip.split('.').map(n => {
        return parseInt(n, 10)
    })

    let [result, base] = [0, 1]

    ip.reverse().forEach(item => {
        result += item * base
        base *= 256
    })

    return result

}


const resolve = (qname, ttl, to) => {

    let r = {}

    r.qname = qname
    r.qtype = 1
    r.qclass = 1
    r.ttl = ttl
    r.rdlength = 4
    r.rdata = to

    return [r]

}


const responseBuffer = query => {
    let question = query.question
    let header = query.header
    let qname = question.qname
    let offset = 16+qname.length
    let length = offset

    for (let i = 0; i < query.rr.length; i++) {
        length += query.rr[i].qname.length+14
    }

    // let buf = new Buffer(length)
    let buf = Buffer.alloc(length)

    header.id.copy(buf, 0, 0, 2)

    buf[2] = 0x00 | header.qr << 7 | header.opcode << 3 | header.aa << 2 | header.tc << 1 | header.rd
    buf[3] = 0x00 | header.ra << 7 | header.z << 4 | header.rcode

    buf.writeUInt16BE(header.qdcount, 4)
    buf.writeUInt16BE(header.ancount, 6)
    buf.writeUInt16BE(header.nscount, 8)
    buf.writeUInt16BE(header.arcount, 10)

    qname.copy(buf, 12)

    question.qtype.copy(buf, 12+qname.length, question.qtype, 2)
    question.qclass.copy(buf, 12+qname.length+2, question.qclass, 2)

    for (let i = 0; i < query.rr.length; i++) {
        let rr = query.rr[i]

        rr.qname.copy(buf, offset)

        offset += rr.qname.length

        buf.writeUInt16BE(rr.qtype, offset)
        buf.writeUInt16BE(rr.qclass, offset+2)
        buf.writeUInt32BE(rr.ttl, offset+4)
        buf.writeUInt16BE(rr.rdlength, offset+8)
        buf.writeUInt32BE(rr.rdata, offset+10)

        offset += 14
    }

    return buf
}


module.exports.response = (query, ttl, to) => {
    let response = {}
    let header = response.header = {}
    let question = response.question = {}
    let rrs = resolve(query.question.qname, ttl, to)

    header.id = query.header.id
    header.ancount = rrs.length

    header.qr = 1
    header.opcode = 0
    header.aa = 0
    header.tc = 0
    header.rd = 1
    header.ra = 0
    header.z = 0
    header.rcode = 0
    header.qdcount = 1
    header.nscount = 0
    header.arcount = 0

    question.qname = query.question.qname
    question.qtype = query.question.qtype
    question.qclass = query.question.qclass

    response.rr = rrs

    return responseBuffer(response)
}

