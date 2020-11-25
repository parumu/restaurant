# Simple Restaurant
## Focus
- Data structure
- Multi-threaded capacity
- Proper unit test
- Functional

## Application
- Accepts following types of requests
  - Add
    - Stores specified items w/ a specified table number
    - For each item, time to cook is assigned on the server side
      - Allowed to generate it randomly 5-15 min or from table?
      - Can update the list w/ cooked orders using add/remove as a trigger

  - Remove
    - Removes a specified item from a specified table

  - Query
    - Returns remaining orders of all items of a table
    - Returns remaining orders of a specified item of a table

- Handles 10+ simultaneous requests

### Data structure

## Client
- Runs on a thread (5-10 simultaneously)
- Randomly sends add/remove/query requests to a table
- Assumes that the set of tables to be a finite set (at least 100)


## API
| Tag | Method | Endpoint | Parameters | Response | Note |
|-----|--------|----------|------------|----------|------|
| Add | POST | /v1/table/[table_id]/items  | items: string[] | 200: Ok | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Remove | DELETE | /v1/table/[table_id]/[item_id] | | 200: Ok | note |
| Query table | GET | /v1/table/[table_id] | | | shows all items of the specified table |
| Query item | GET | /v1/table/[table_id]/[item_id] | | | show the number of the specified items of the specified table |

- 1 <= table_id <= num_tables

## How to build
1. install nightly version of Rust with `rustup`
```
$ cd [project root]
$ cargo build
```

## How to run
```
$ cd [project root]
$ cargo run
```

## Expected outputs

# How to deploy (needed?)
- add Dockerfile and docker-compose.yaml