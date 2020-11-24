# Higher level requirements
- Focus on
  - Data structure choice
  - API design
  - Data correctness
  - Multi-threaded capacity
  - Proper unit testing
  - Making it appropriately functional
  - Data manipulation techniques

- Should be
  - Deployable
  - Documented

- Add README
  - How to build
  - How to run
  - Expected outputs

## Ok to use
- Libraries that deal with
  - threading
  - thread channels
  - TCP/IP streaming
  - HTTP processing
  - REST webserver endpoints
  - other functionality that bring the data into your app

  - example accepted libs
    - hyper (HTTP)
    - threadpool (threading)
    - rocket or iron (webserver)
    - serde/json (for JSON serialization)
    - riker (actor system)
    - futures/tokio (futures)
    - lazy_static (statics which can be initialized at runtime)
    - std

# Lower level requirements

- Create a restaurant application
  - accepts menu items from various serving staff
  - stores the item with countdown (# of min to be served)
  - can give a quick snapshot of
    - any
    - all items on its list
  - can remove orders

- components
  - application
    - Upon add req, store
      - item
      - table number
      - time to cook

    - Upon remove, remove a specified item for a specified table number

    - Upon query, show
      - all items for a specified table number
      - a specified item for a specified table number

    - handle 10+ simultaneous incoming add/remove/query requests

    - MAY assign time to cook randomly (5-15 min)
    - MAY: no need to update time to cook
      - can update the list add/remove as the trigger

  - client (multiple)
    - add, remove, query menu items for each table
    - add 1+ items w/ a table number
    - remove an item for a table
    - query the items still remaining for a table
    - MAY assume the set of tables to be a finite set (at least 100)

## Allowed Assumptions

- tables and items can be identified in any way
  - can use int for tables (in example)
  - can use int or string for items (probably)

- “Clients” can be simple threads created in main()
  - 5-10 running simultaneously

- Acceptable API
 - HTTP REST
 - direct API calls as long as they mimic HTTP REST-like API
   - api_call1(string id, string resource)


