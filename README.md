# Template Repo

This repo is the foundation for creating a new microservice in the yabs project.

## Using this repo
1. Create a new repo on github and select this as the template.
2. Adjust the name of this app in the Cargo.toml and the Dockerfile entry line.

## Creating a release

```sh
cargo release patch/minor/major --execute
````

##  Creating a sql migration

```sh
sqlx migrate add
```

## Update sql scripts

```shell
cargo sqlx prepare
```