(import
  (
    let lock = builtins.fromJSON (builtins.readFile ./flake.lock); in
    fetchTarball {
      url = with lock.nodes.flake-compat.locked;
        "https://github.com/${owner}/${repo}/archive/${rev}.tar.gz";
      sha256 = lock.nodes.flake-compat.locked.narHash;
    }
  )
  { src = ./.; }
).shellNix
