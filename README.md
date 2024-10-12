# transqsh 

a music transcoder to 96k opus. it just does the thing.

## todo

- [X] fix the memory leak
- [X] metadata
- [ ] really fix the memory leak
- [ ] get to 0 fails on our library (at 35 fails @ 2024-10-12)
- [ ] makes "./out.opus"??

## usage

well hi there!

- theres a nix flake, you can install that.
- if you can't install that you could `cargo build --release` and copy the output to a /usr/local/bin.
- you could also just clone the repo and `cargo run --release -- --help`

poke me if you need help, see [my website](https://mei.puppycat.house/) for how

## license

This work is free. You can redistribute it and/or modify it under the
terms of the Do What The Fuck You Want To Public License, Version 2,
as published by Sam Hocevar. See the `LICENSE` file for more details.
