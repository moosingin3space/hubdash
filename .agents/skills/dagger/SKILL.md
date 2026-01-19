---
name: dagger
description: Explains how to use Dagger. Consult this skill if you need to know how to use Dagger.
---
# Dagger

We use Dagger as a CI/CD environment in order to ensure consistency between local development
and GitHub Actions. Here is a Dagger cheat sheet:

## What can it do?

```bash
dagger functions
```

## How to invoke?

```bash
dagger call [FUNCTION] [ARGS...]
```

`FUNCTION`, in this case, refers to one of the functions from above. Many Dagger functions do not
require any arguments.

## Standard arguments

Most functions exposed will take a `source` argument, which can be passed a directory in the repo.
