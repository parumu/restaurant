# #![feature(decl_macro, proc_macro_hygiene)]
- decl_macro
```
error[E0658]: `macro` is experimental
  --> src/main.rs:56:1
   |
56 | #[get("/table/<table_id>/item/<item_uuid>")]
   | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: see issue #39412 <https://github.com/rust-lang/rust/issues/39412> for more information
   = help: add `#![feature(decl_macro)]` to the crate attributes to enable
   = note: this error originates in an attribute macro (in Nightly builds, run with -Z macro-backtrace for more info)
```

- proc_macro_hygiene
```
error[E0658]: procedural macros cannot be expanded to expressions
  --> src/main.rs:69:7
   |
69 | /       routes![
70 | |         add_items,
71 | |         remove_item,
72 | |         get_all_items,
73 | |         get_item,
74 | |       ],
   | |_______^
   |
   = note: see issue #54727 <https://github.com/rust-lang/rust/issues/54727> for more information
   = help: add `#![feature(proc_macro_hygiene)]` to the crate attributes to enable
```