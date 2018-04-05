

const fs = require('fs')
const test = require('ava')
const path = require('path')
const writeLog = require('./../bin/log')
const logPath = path.join(__dirname, './../log/updns.log')


test('Write log', async t => {

    writeLog(1)

    await setTimeout(() => {
        writeLog(2)
    }, 10)

    const logs = new Promise((success, fail) => {

        setTimeout(() => {

            fs.readFile(logPath, {
                encoding: 'utf8'
            },(err, content) => {
    
                if(err) fail()
                else success(content.split('\n').length)

                fs.unlinkSync(logPath)
    
            })

        }, 100)

    })

    t.is(await logs, 3)

})


