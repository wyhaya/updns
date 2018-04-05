

const test = require('ava')


// Test whether the DNS service can be created


test.cb('Create DNS services', t => {

    const updns = require('./../lib/index').createServer(1234)

    updns.on('listening', () => {
        t.pass()
        t.end()
    })

    updns.on('error', err => {
        t.fail(`DNS service creation failure : ${err}`)
        t.end()
    })

})

test.todo('Normal return of data')


