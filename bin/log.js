

const fs = require('fs')
const path = require('path')
const logConfigPath = path.join(__dirname, './../log/updns.log')


function write(log) {

    fs.appendFile(logConfigPath, log, {
        encoding: 'utf8'
    }, err => {

        if(err && err.code === 'ENOENT'){

        }

    })

}


module.exports = (domain) => {

    const log = `${new Date().toString()}    ${domain}\n`

    fs.stat(path.join(__dirname, './../log'), (err, stat) => {

        if(err && err.code === 'ENOENT'){
            fs.mkdir(
                path.join(__dirname, './../log'),
                err => {
                    if(err) {
                        return
                    }else write(log)
                }
            )
        } else write(log)
    })
    
}


