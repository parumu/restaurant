const axiosBase = require('axios')
const axios = axiosBase.create({
  baseURL: 'http://localhost:8888',
  headers: {
    'Content-Type': 'application/json',
    'X-Requested-With': 'XMLHttpRequest'
  },
  responseType: 'json'
})
const yargs = require('yargs/yargs')
const { hideBin } = require('yargs/helpers')

async function startClientFor(numTables, clientId) {
  const tableId = clientId

  const logError = (s) => console.error(`[Error] ${tableId}:`, s)
  const logWarn = (s) => console.warn(`[Warn] ${tableId}:`, s)
  const logInfo = (s) => console.error(`[Info] ${tableId}`, s)

  while(true) {
    const tableId = Math.floor(Math.random() * numTables)

    let uuid
    try {
      // add 1 items
      let res = await axios.post(`v1/table/${tableId}/items`, {
        item_names: [
          `${tableId}-dish`,
        ],
      })
      logInfo(`To table ${tableId}, added item: ${JSON.stringify(res.data)}`)
      uuid = res.data[0].uuid

    } catch(error) {
      if (error.response.status === 429) { // TooManyRequest
        logError(`Table ${tableId} is full`)
      } else {
        logError(error)
      }
      continue
    }

    // get added item
    try {
      const res = await axios.get(`/v1/table/${tableId}/item/${uuid}`)
      logInfo(`From table ${tableId}, got: ${JSON.stringify(res.data)}`)

    } catch(error) {
      if (error.response.status === 404) { // NotFound
        logWarn(`Tried to get item w/ ${uuid}, but item is missing`)
        continue
      } else {
        logError(error)
      }
    }

    // get all items of table
    try {
      const res = await axios.get(`/v1/table/${tableId}/items`)
      logInfo(`Got all items of table ${tableId}: ${JSON.stringify(res.data)}`)

    } catch(error) {
      logError(error)
    }

    // remove added item
    try {
      while(true) {
        await axios.delete(`/v1/table/${tableId}/item/${uuid}`)
        logInfo(`From table ${tableId}, removed item w/ uuid ${uuid}`)
        break
      }
    } catch(error) {
      if (error.response.status === 404) { // NotFound
        logWarn(`Tried to remove item w/ ${uuid}, but item is missing`)
        continue
      } else {
        logError(error)
      }
    }
  }
}

async function main(numTables, numClients) {
  for(let i=0; i<numClients; i++) {
    startClientFor(numTables, i)
  }
}
const args = yargs(hideBin(process.argv))
  .option('clients', {
    alias: 'c',
    description: 'Number of client threads',
    default: 10,
  })
  .option('tables', {
    alias: 't',
    description: 'Number of tables at restaurant',
    default: 100,
  })
  .help()
  .argv

main(args.tables, args.clients)