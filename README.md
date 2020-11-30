# Simple Restaurant

## Application
- Simulates a restaurant w/ tables
- Maintains a list of outstanding orders for each table
- Randomly assigns the time to cook (5-15 minutes) to each order
- Accepts below HTTP requests
  - Add
    - Adds orders to a specified table

  - Remove
    - Removes an order of a specified UUID

  - Query item
    - Returns an order of a specified UUID

  - Query all items
    - Returns outstanding orders of a specified table

### Requirements
- `rocket` requires nightly version of Rust

### How to build/run
#### Cargo
```
$ cd [Project root]
$ cargo run --bin application
```

#### Docker
```
$ cd [Project root]
$ docker-compose up --build
```

### Configuration
`[Project root]/Rocket.toml`

| name | description |
|------|-------------|
| address | Address that the application listens to |
| port | Port that the application listens to |
| num_tables | # of tables in the restaurant |
| max_table_items | Maximum # of outstanding orders that a table can have |
| one_min_in_sec  | # of seconds that constitutes 1 minute |
| log | Rocket log level. Valid values are: "normal", "debug", or "critical" |
| secret_key | Rocket secret_key that is a 256-bit base64 encoded string. Required for production |

### API
| Tag | Method | Endpoint | Parameters | Response | Description |
|-----|--------|----------|------------|----------|------|
| Add | POST | /v1/table/[table_id]/items  | item_names: string[] | 200: Ok(Item[]), 429: TooManyItems (max item exceeded), 406: NotAcceptable (bad table id) | Adds items w/ specified names to the specified table and returns added items |
| Remove | DELETE | /v1/table/[table_id]/item/[uuid] | | 200: Ok, 404: NotFound, 406: NotAcceptable | Removes an item of the specified UUID |
| Query table | GET | /v1/table/[table_id]/items | | 200: Ok(Item[]), 406: NotAcceptable | Returns all items of the specified table that is being cooked |
| Query item | GET | /v1/table/[table_id]/item/[uuid] | | 200: Ok(Item), 406: NotAcceptable | Returns an item of the specified UUID |

#### Note
- 0 <= `table_id` < num_tables
- Item object schema:
    ```
    {
      uuid: string,
      name: string,
      table_id: number,
      created_at: number,
      ready_at: number,
      is_removed: boolean,
    }
    ```

### Architecture

```
Rocket HTTP Server -> OrderMgr
```
- HTTP server directly forwards each request to a corresponding public method of `OrderMgr`
- `OrderMgr` is in charge of maintaining the list of outstanding orders of each table

### OrderMgr
- Maintains outstanding orders of each table with `TableOrder`
- Stores `TableOrder`s in a `Vector`
- `TableOrder` maintains a priority queue of outstanding orders with the order with minimum
  `ready_at` at the root
- `TableOrder` also maintains a hash table of outstanding orders with order `UUID`
  as the key and the `Item` (order) as the value
- An Item is shared by the priority queue and hash table
- When a client request is made, Items whose `ready_at` is older than or equal to
  now is popped out of the priority queue and also removed from the hash table

```
    Vector
    +-----------+-----+
    | (Table 0) | ... |
    +-----------+-----+
         |
         V
      TableOrder ---------------+               ..
         |                      V              /
         +---> Hash table     Priority queue [ ]
               +-------+                    /  \
               | UUID1 | ---> Item 1 <--- [ ]  [ ]
               +-------+                  / \
               |  ...  |                ..   ..
               +-------+
```

### Running time
- Looking for a table in a Vector is O(1)
- Adding an item to priority queue is O(log n) since priority queue is BinaryHeap
- Adding an item to hash table is O(1)
- Popping out an item is O(log n) due to using BinaryHeap
- Removing an item is O(1):
  - Removing from hash table is O(1)
  - An item is not removed from priority queue, but marked as removed. This is also O(1).
    The item will stay in priority queue until it's popped out
- Getting an item is O(1) since the item is obtained from hash table
- Getting all items is O(n) since it gets all values from hash table

#### Unresolved issues
1. `RefCell` is used in multi threaded context and the lock is done at `TableOrders`
   level for all operations because of that.
   - Each `Item` object needs to be owned by `BinaryHeap` and `HashMap` in `TableOrders`.
     A wrapper type that does reference counting GC is needed.
   - Since `TableOrders` is used by `OrderMgr` and `OrderMgr` runs on multiple threads,
     the reference counting wrapper needs to be `Arc` instead of `Rc`.
   - `Arc` doesn't expose mutable reference of the contained type, but `Item` needs to be
     modified upon deletion. Therefore `Arc` needed to be wrapped by something that exposes
     mutable reference.
   - `RefCell` and `Mutex` exposes mutable refernce. `RefCell` is designed for single-threaded
     environment and `Mutex` is designed for multi-threaded environment.
   - Since `Item` is used in multi-threaded environment, `Mutex` should be used.
     But `Mutex` cannnot be used since `BinaryHeap` requires `Ord` to the contained type,
     but `Mutex` doesn't implement `Ord`. So, `RefCell` needs to be used.

   - To use `RefCell`, `Sync` and `Send` unsafe traits are implemented to `TableOrders`.
     `TableOrders` is locked for for all operations including queries although
     read lock should suffice for queries.

2. To unwrap `Arc<RefCell<Item>>` to `Item`, new `Item` is manually created.

### Note
- Changed to update the list of items being cooked not only by add and remove requests, but also with query item and query all items requests.

## Client
- Runs specified number of client threads
- The number of tables is hardcoded to 100
- Endlessly executes below sequence on each client thread:
  1. Select a table
  2. Add 1 item to the table
  3. Get the added item from the table
  4. Get all items of the table
  5. Remove the added item from the table

### Requirements
- On Linux, `reqwest` requires OpenSSL 1.0.1, 1.0.2, 1.1.0, or 1.1.1 with headers
  - `OPENSSL_LIB_DIR` and `OPENSSL_INCLUDE_DIR` need to be exported

#### Unresolved issues
1. Very frequently client requests fail with `hyper::Error(IncompleteMessage): connection closed before message completed`.
   No error observed on the application side.

### How to build/run
```
$ cd [Project root]
$ cargo run --bin client -- -t 100 -c 10  # run with 100 tables and 10 client threads
```

## Expected output
### Application
- All logs for the requests made by client threads
### Client
- Error and warning logs of the client thread requests