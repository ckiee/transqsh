{ pkgs ? import <nixpkgs> { } }:

with pkgs;

mkShell {
  buildInputs = [ pkg-config ffmpeg_6-full ];
  # HACK:
  inputsFrom = [ ffmpeg_6-full ];

  LIBCLANG_PATH = "${llvmPackages.libclang.lib}/lib";
}
