

const test = require('ava')
const fs = require('fs')
const path = require('path')
const daemon = require('./../bin/daemon')


test.cb('Creating a daemon', t => {

    t.plan(2)

    const testFilePath = path.join(__dirname, './process.test.js')

    fs.writeFileSync(testFilePath, `
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


test.todo('Spawn process')

// test.cb('Spawn process', t => {

//     t.plan(1)
//     daemon.spawn('vim', path.join(__dirname, './index.test.js')).then(child => {
//         console.log(child)
//         t.end()
//     }).catch(e => {

//     })
    
// })


