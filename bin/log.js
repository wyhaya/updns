

const fs = require('fs')
const path = require('path')
const logConfigPath = path.join(__dirname, './../log/updns.log')


const echo = log => {

    fs.appendFile(logConfigPath, log, {
        encoding: 'utf8'
    }, err => {

        if(err && err.code === 'ENOENT'){
            return
        }

    })

}


const getDateTime = () => {

    const date = new Date()
    let y = date.getFullYear().toString()
    let m = (date.getMonth() + 1).toString().padStart(2, '0')
    let d = date.getDate().toString().padStart(2, '0')
    let hr = date.getHours().toString().padStart(2, '0')
    let mi = date.getMinutes().toString().padStart(2, '0')
    let se = date.getSeconds().toString().padStart(2, '0')
    let ms = date.getMilliseconds().toString().padStart(3, '0')

    return `${y}-${m}-${d} ${hr}:${mi}:${se}.${ms}`

}


const write = domain => {

    const log = `${getDateTime()}    ${domain}\n`

    fs.stat(path.join(__dirname, './../log'), err => {

        if(err && err.code === 'ENOENT'){
            fs.mkdir(
                path.join(__dirname, './../log'),
                err => {
                    if(err) {
                        return
                    }else echo(log)
                }
            )
        } else echo(log)
    })
    
}


module.exports = {
    getDateTime,
    write
}