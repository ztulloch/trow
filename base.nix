with import <nixpkgs> {};
let
  pkgs = import <nixpkgs> {};
  date = "2017-11-15";
  mozilla-overlay = fetchFromGitHub {
    owner = "mozilla";
    repo = "nixpkgs-mozilla";
    rev = "661f3f4d8183f493252faaa4e7bf192abcf5d927";
    sha256 = "0g1ig96a5qzppbf75qwll8jvc7af596ribhymzs4pbws5zh7mp6p";
  };
  mozilla = (import mozilla-overlay) pkgs pkgs;
  rust-nightly = mozilla.rustChannelOf { date = date; channel = "nightly"; };
in
stdenv.mkDerivation rec {
  name = "lycaon-base";
  env = buildEnv { name = name; paths = propogatedBuildInputs; };
  propogatedBuildInputs = [
    pkgconfig
    rust-nightly.rust
    cmake
    perl
    go
    # for .proto generation
    protobuf

    # dev
    bash
    coreutils
  ];
  buildInputs = [ makeWrapper ];
  PATH = builtins.foldl' (x: y: "${x}:${y}/bin") "/dev/null" propogatedBuildInputs;
  wrapper = writeText "wrapper" ''
    export PATH=${PATH}
    exec ${bash}/bin/bash
  '';

  phases = [ "installPhase" ];
  installPhase = ''
    mkdir -p $out/bin
    ln -s ${bash}/bin/bash $out/bin/entrypoint.sh
    wrapProgram $out/bin/entrypoint.sh --prefix PATH : "${PATH}"
    # cp ${wrapper} $out/bin/entrypoint.sh
    # chmod +x $out/bin/entrypoint.sh
  '';

  shellHook = ''
    echo Rust Nightly: ${date}
  '';
}
