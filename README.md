# Simple Restaurant

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

### Configuration
Configuration file is `Rocket.toml` in project root directory
- address = "0.0.0.0"
- port = 8888
- num_tables = 100
- max_table_items = 10
- one_min_in_sec = 1  // should be 60 for the real scenario
- log = "normal" // rocket log level - "normal", "debug", or "critical"
- secret_key = [a 256-bit base64 encoded string] // rocket secret_key. required for production

### API
| Tag | Method | Endpoint | Parameters | Response | Note |
|-----|--------|----------|------------|----------|------|
| Add | POST | /v1/table/[table_id]/items  | item_names: string[] | 200: Ok(Item[]), 429: TooManyItems (max item exceeded), 406: NotAcceptable (bad table id) | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Remove | DELETE | /v1/table/[table_id]/item/[item_id] | | 200: Ok, 404: NotFound, 406: NotAcceptable | note |
| Query table | GET | /v1/table/[table_id]/items | | 200: Ok(Item[]), 406: NotAcceptable | shows all items of the specified table |
| Query item | GET | /v1/table/[table_id]/item/[item_id] | | 200: Ok(Item), 406: NotAcceptable | show the number of the specified items of the specified table |

#### Note
- 0 <= table_id < num_tables
- Item object schema:
    ```
    {
      uuid: string,
      name: string,
      table_id: number,
      created_at: number,
      ready_at: number,
      pub is_removed: boolean,
    }
    ```

### Specification change
- Changed to update active orders with query table/item requests as triggers as well.
  Originally using RwLock so that query requests don't interface each other.
  Replaced that with Mutex so that query requests reflect actual state of orders.
  Because of this change, query request became slower.

### Architecture

### Data structure

#### Why Arc?
- Each `Item` object needs to be shared by `BinaryHeap` and `HashMap` in `TableOrders`.
  Reference counting GC is needed for that.
- Since `TableOrders` is used by `OrderMgr` and `OrderMgr` runs on multiple threads,
  the reference counting GC needs to be `Arc` instead of `Rc`.

#### Why RefCell?
- Since `Item` needs to be wrapped by `Arc`, but `Arc` doesn't expose mutable reference
  of contained type, Item is further needed to be wrapped by something that exposes
  mutable reference.
- `RefCell` and `Mutex` can be used for the purpose. `RefCell` is designed for single-threaded
  environment whereas `Mutex` is for multi-threaded.
- Since `Item` is used in multi-threaded environment, `Mutex` should be used, but
  `Mutex` cannnot be used since `BinaryHeap` which requires `Ord` to the contained type
  needs to hold `Item` but `Mutex` doesn't implement `Ord`.

#### Remaining issues
1. Many requests from client fails with `hyper::Error(IncompleteMessage)` on the client side.
   There is no error on the server side.
1. `RefCell` is used in multi threaded context.
2. Manually implementing `Sync` and `Send` traits to `TableOrders` to use `RefCell`.
   For thread safely, `Mutex` is used to guard all accesses to `Item` in `OrderMgr`.
3. To unwrap `Arc<RefCell<Item>>` to `Item`, new `Item` is manually created.

## Client
- Runs on a thread (5-10 simultaneously)
- Randomly sends add/remove/query requests to a table
- Assumes that the set of tables to be a finite set (at least 100)

## Required environment
- nightly rust e.g.
```
$ rustup install nightly
$ rustup default nightly
```
- On Linux, OpenSSL 1.0.1, 1.0.2, 1.1.0, or 1.1.1 with headers
```
$ sudo apt install libssl-dev

# dpkg -L libssl-dev | grep lib
# dpkg -L libssl-dev | grep include

$ export OPENSSL_LIB_DIR=/usr/lib/x86_64-linux-gnu
$ export OPENSSL_INCLUDE_DIR=/usr/include/openssl
```

## How to start
### Cargo
```
$ cd [project root]
$ cargo run --bin application
$ cargo run --bin client 50   # 50 is # of clients
```

### Docker
```
$ cd [project root]
$ docker-compose up
```

## Expected outputs
