# transqsh 

a music transcoder to 96k opus and a few other codecs. it just does the thing.

## todo

- [X] fix the memory leak
- [X] metadata
- [-] cover art copy
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
| codec | performance                               |
|:------|:------------------------------------------|
| opus  | `Transcoded 95.8 GB ⇒ 27.9 GB (-70.89%)`  |
| aac   | `Transcoded 95.8 GB ⇒ 36.4 GB (-62.04%) ` |
| mp3   | `Transcoded 95.8 GB ⇒ 61.6 GB (-35.70%)`  |

https://pdfs.semanticscholar.org/cb36/5ed1cdc02e1b250cc7ff5a9ee890d863204d.pdf#page=6

https://wiki.hydrogenaud.io/index.php?title=AAC_encoders

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
