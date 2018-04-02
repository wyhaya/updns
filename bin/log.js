

const fs = require('fs')
const path = require('path')
const logConfigPath = path.join(__dirname, './../log/updns.log')


module.exports = (domain) => {

    const log = `${domain}\n`

    fs.appendFile(logConfigPath, log, {
        encoding: 'utf8'
    }, err => {

        if(err){
            if(err.code === 'ENOENT'){
                fs.mkdirSync(path.join(__dirname, './../log'))
            }else {
                process.send({
                    state: 0, 
                    message: err
                })
            }
        }else {

            process.send({
                state: 1, 
                message: log
            })

        }

    })
    
}


