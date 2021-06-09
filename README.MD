# Password Generator Bot
A telegram bot to generate secure random passwords for you written in Rust.

## Features
* **Lightweight**: Very low memory usage thanks to Rust!
* **Secure**: Use `crypto/rand` to generate secure random passwords.
* **No trace**: Nothing is stored on server and no logs.

## Running
At first clone this project and then run `cargo build --release`. It will create an executable file in target folder.

Then set `TELOXIDE_TOKEN` as your bot token in an environmental variable, and just run the compiled program.

There are also binaries available in releases.

## Live Bot
I try to keep this bot alive.

http://t.me/pass_generator_bot