

const fs = require('fs')
const test = require('ava')
const path = require('path')
const writeLog = require('./../bin/log')
const logPath = path.join(__dirname, './../log/updns.log')


test('Write log', async t => {

    writeLog(Date.now())

    await new Promise(waiting => setTimeout(() => {
        for(let i = 0; i < 8; i++){
            writeLog(Date.now())
        }
        waiting()
    }, 10))

    const logs = new Promise((success, fail) => setTimeout(() => {

        fs.readFile(logPath, {
            encoding: 'utf8'
        }, (err, content) => {

            fs.unlinkSync(logPath)

            if (err) fail()
            else success(content.split('\n').length)

        })

    }, 100))

    t.is(await logs, 10)

})


