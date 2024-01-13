# Changelog

## 0.13.0

* Update keyring-rs, more explicitly state platform support. Linux has full support, macOS and Windows are untested but could work.


## 0.12.1 2024-01-12

* Upload changelog

## 0.12.0 2024-01-12

Significant improvements have been made. Debug logs are available when running with `RUST_LOG=DEBUG` set as an environment variable. 

* Support for cloning from local repositories (almost any Git URL should work now). This also separates some more logic.
* `run` now works with a deploy path when running an archive.
* `network` option has been removed, the context now determines this (although it's not yet implemented). 

## 0.11.2 2023-12-15

* Fix exe name being overwritten by default value from CLI. Fix run state precedence.

## 0.11.1 2023-12-15

* Upload changelog.

## 0.11.0 2023-12-15

* Deploy and run function added back in. Not compatible with previous versions. 

## 0.10.0 2023-12-15

* Most features have been temporarily removed during the refactor. Use v0.9.0 if they are needed. This release contains:
    - A major refactor of how state is loaded, to streamline it across commands
    - A revamped "auth" command, which now uses a key and by default scopes only to the repo
    - A revamped "run" command, which now utilizes the general set state

## 0.9.0 2023-12-14

* dployer is the default option now, dployer_env renamed to env_var and new "run" subcommand

## 0.8.0 2023-12-12

* Add support for dployer, which allows secret loading to also be handled externally by using named pipe communication with Docker containers

## 0.7.0 2023-12-11

* Set default repo to default_git_root_origin and check for git_root_origin in order to read URL from current repo

## 0.6.0 - 2023-12-11

* Rewrite Git history to fix commit author
* Remove debug URL print

## 0.5.0 - 2023-12-11

* Separated primitive errors into file
* Fixed extract not having correct path
* Fixed setting password not getting correct name from repo URL

## 0.4.0 - 2023-12-11

