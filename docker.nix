with (import <nixpkgs> {});

pkgs.dockerTools.buildImage rec {
  name = "lycaon-base";
  tag = "latest";

  contents = import ./base.nix;

  config = {
    Cmd = ["${contents}/bin/entrypoint.sh"];
  };
}
