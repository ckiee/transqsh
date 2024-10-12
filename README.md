# transqsh 

a music transcoder to 96k opus. it just does the thing.

## todo

- [X] fix the memory leak
- [X] metadata
- [ ] really fix the memory leak
- [ ] get to 0 fails on our library (at 35 fails @ 2024-10-12)
- [ ] makes "./out.opus"??
- [ ] sync deletions

## usage

well hi there!

- theres a nix flake, you can install that.
- if you can't install that you could `cargo build --release` and copy the output to a /usr/local/bin.
- you could also just clone the repo and `cargo run --release -- --help`
    + you need ffmpeg 6, ideally with fdk_aac if you plan to use that.

poke me if you need help, see [my website](https://mei.puppycat.house/) for how

### codecs
as of 2024-10-12 on my ~/Music/flat
- mp3: `Transcoded 95.8 GB ⇒ 61.6 GB (-35.70%)`
- opus: `Transcoded 95.8 GB ⇒ 27.9 GB (-70.89%)`

### syncthing that shit

you can get the transcoded files onto your phone with a systemd timer and syncthing folder.

it's pretty simple. see [my nixos module](https://github.com/ckiee/nixfiles/tree/97313d61e0e83ca84251fbce572cbd247ced92bb/modules/services/transqsh.nix) for reference.

my stack:
- desktop: transqsh, [syncthing](https://syncthing.net/), systemd.{[service](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html),[timer](https://www.freedesktop.org/software/systemd/man/latest/systemd.timer.html)}
- iPhone: [mobius sync](https://mobiussync.com/), [doppler](https://brushedtype.co/doppler/)

## license

This work is free. You can redistribute it and/or modify it under the
terms of the Do What The Fuck You Want To Public License, Version 2,
as published by Sam Hocevar. See the `LICENSE` file for more details.
