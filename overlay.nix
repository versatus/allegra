final: prev:
let
  f = pname: prev.callPackage ./package.nix { inherit pname; };
in
{
  server = f "server";
  cli = f "cli";
  vmm = f "vmm";
  monitor = f "monitor";
  broker = f "broker";
  state = f "state";
  quorum = f "quorum";
  network = f "network";
}
