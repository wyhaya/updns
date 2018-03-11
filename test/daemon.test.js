

const test = require('ava')
const fs = require('fs')
const path = require('path')
const daemon = require('./../lib/daemon')

test.cb('Creating a daemon', t => {

    t.plan(2)

    const testFilePath = path.join(__dirname,'./process.test.js')

    fs.writeFileSync(testFilePath,`
        process.send({
            message: 'ok'
        })
    `)

    const child = daemon.createProcess(testFilePath)

    t.is(typeof child.pid, 'number')

    child.on('message', data => {

        daemon.killProcess(child.pid)
        fs.unlinkSync(testFilePath)
        t.is(data.message, 'ok')
        t.end()

    })
    

})


