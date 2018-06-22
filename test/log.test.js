

const fs = require('fs')
const test = require('ava')
const path = require('path')
const log = require('./../bin/log')
const logPath = path.join(__dirname, './../log/updns.log')


test('Write log', async t => {

    Number.prototype[Symbol.iterator] = function * () {
        for(let i = 0; i <= this - 1; i++) yield i
    }

    const count = [...1000]
    log.write(Date.now())

    await new Promise(waiting => setTimeout(() => {
        count.map(() => log.write(Date.now()))
        waiting()
    }, 10))

    const logs = new Promise(success => setTimeout(() => {

        let content = ''
        content = fs.readFileSync(logPath, {
            encoding: 'utf8'
        })
        fs.unlinkSync(logPath)
        success(content.split('\n').length)

    }, 100))

    t.is(await logs, count.length + 2)

})


test('Get date time', t => {

    const [data, time] = log.getDateTime().split(' ')

    t.is(/^\d{4}-(0[1-9]|1[0-2])-((0|1|2)[0-9]|3[0-1])$/.test(data), true) // ?
    t.is(/^(\d{2}:){2}\d{2}.\d{3}$/.test(time), true)

})


