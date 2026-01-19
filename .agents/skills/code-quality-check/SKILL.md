---
name: code-quality-check
description: Used to assert that our Rust code follows conventions. Run after each change to ensure our code maintains a high level of quality.
---

# Code Quality Check

We make use of basic Rust tools, like `cargo clippy` and `cargo fmt`.
In order to run them identically to how we run them in CI/CD, use Dagger:

```bash
dagger call lint --source=.
```

Note that you can still run them from the CLI like normal, which will
generally be faster, but using them in Dagger will guarantee consistency.
