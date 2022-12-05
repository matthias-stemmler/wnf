# `wnf` usage examples

Run an example using

```shell
cargo run --all-features --example <example-name>
```

e.g.
```shell
cargo run --all-features --example recipes_get_set
```

There are two kinds of examples in this folder:

## Apps (named `apps_*.rs`)

These demonstrate real-world use cases. They are focused on the actual application rather than showing how to use
specific methods.

## Recipes (named `recipes_*.rs`)

These demonstrate how to use specific methods. They register a subscriber for `tracing` events so you can see a trace of
all API calls and invocations of subscription callbacks in the console.
