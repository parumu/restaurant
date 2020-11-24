# Requirements
- Create a restaurant application
  - accepts menu items from various serving staff
  - stores the item with countdown (# of min to be served)
  - can give a quick snapshot of
    - any
    - all items on its list
  - can remove orders

  - components
    - application
      - Upon add, store
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
        - can update the list add/remove as a trigger

    - client (multiple)
      - add, remove, query menu items for each table
      - add 1+ items w/ a table number
      - remove an item for a table
      - query the items still remaining for a table
      - MAY assume the set of tables to be a finite set (at least 100)

# API
| Tag | Method | Endpoint | Parameters | Note |
|-----|--------|----------|------------|------|
| Add | POST | /v1/[table]/item  | | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Add bulk | POST | /v1/[table]/items  | items: string[] | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Remove | DELETE | /v1/[table]/[item] | |
| Query table | GET | /v1/[table] | | shows all items of the specified table |
| Query item | GET | /v1/[table]/[item] | | show the number of the specified items of the specified table |
|

# Others
- Focus on
  - Data structure
  - Multi-threaded capacity
  - Proper unit test
  - Functional

- Should be
  - Deployable --> Docker?

- Add README
  - How to build
  - How to run
  - Expected outputs
  - Document XYZ...

