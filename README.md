# Simple Restaurant
## Application
- Requests
  - Add
    - Store items w/ a table number
    - Assign time to cook on the server side
      - Allowed to generate it randomly 5-15 min or from table?
      - Can update the list w/ cooked orders using add/remove as a trigger

  - Remove
    - Remove a specified item for a specified table number

  - Query
    - Return all items of a table
    - Return the number of outstanding orders of a specified item for a table

- Should be able to handle 10+ simultaneous requests

## Client
- Add 1+ items w/ a table number
- Remove an item for a table
- Query the items still remaining for a table
- MAY assume the set of tables to be a finite set (at least 100)
- Run in 5-10 threads simultaneously

# API
| Tag | Method | Endpoint | Parameters | Note |
|-----|--------|----------|------------|------|
| Add | POST | /v1/[table]/item  | | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Add bulk | POST | /v1/[table]/items  | items: string[] | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Remove | DELETE | /v1/[table]/[item] | |
| Query table | GET | /v1/[table] | | shows all items of the specified table |
| Query item | GET | /v1/[table]/[item] | | show the number of the specified items of the specified table |
|

# How to build

# How to run

# Expected outputs

# How to deploy (needed?)

# Others
- Focus on
  - Data structure
  - Multi-threaded capacity
  - Proper unit test
  - Functional