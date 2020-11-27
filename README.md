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
- address = "0.0.0.0"
- port = 8888
- num_tables = 100
- max_table_items = 10
- one_min_in_sec = 60
- log = "normal" // rocket log level - "normal", "debug", or "critical"
- secret_key = [a 256-bit base64 encoded string] // rocket secret_key. required for production

### API
| Tag | Method | Endpoint | Parameters | Response | Note |
|-----|--------|----------|------------|----------|------|
| Add | POST | /v1/table/[table_id]/items  | item_names: string[] | 200: Ok Item[] | time2cook is randomly assigned on server side. returns an id associated with the added items |
| Remove | DELETE | /v1/table/[table_id]/item/[item_id] | | 200: Ok | note |
| Query table | GET | /v1/table/[table_id]/items | | Item[] | shows all items of the specified table |
| Query item | GET | /v1/table/[table_id]/item/[item_id] | | Item | show the number of the specified items of the specified table |

- 1 <= table_id <= num_tables

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
1. `RefCell` is used in multi threaded context.
2. Manually implementing `Sync` and `Send` traits to `TableOrders` to use `RefCell`.
   For thread safely, `RwLock` is used to guard all accesses to `Item` in `OrderMgr`.
3. To unwrap `Arc<RefCell<Item>>` to `Item`, new `Item` is manually created.

## Client
- Runs on a thread (5-10 simultaneously)
- Randomly sends add/remove/query requests to a table
- Assumes that the set of tables to be a finite set (at least 100)

## Required environment
- Nightly Rust

## How to build
### Cargo
1. install nightly version of Rust with `rustup`
```
$ cd [project root]
$ cargo build
```

### Docker
```
$ cd [project root]
$ docker build -t restaurant .
```

## How to run
### Cargo
```
$ cd [project root]
$ cargo run --release
```

### Docker
```
$ cd [project root]
$ docker-compose up
```

## Expected outputs
