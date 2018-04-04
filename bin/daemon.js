

const child_process = require('child_process')


module.exports.createProcess = filePath => {

    let argvs = [...process.argv].slice(1, process.argv.length)

    let options = {
        stdio: 'ignore',
        env: process.env,
        cwd: process.cwd(),
        detached: true
    }

    let child = child_process.fork(filePath, argvs, options)

    child.unref()

    return child

}


module.exports.killProcess = pid => {

    process.kill(pid)

}


module.exports.spawn = (processName, argv) => {

    return new Promise((suc, err) => {

        const child = child_process.spawn(processName, [ argv ], {
            stdio: 'inherit'
        })

        child.on('error', err)

        child.on('exit', (code, signal) => {
            code === 0 ? suc(child) : err(signal)
        })

    })

}


