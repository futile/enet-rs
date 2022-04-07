{
  description = "Flake for building enet-rs using nix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/release-21.11";
  };

  outputs = { self, nixpkgs }@inputs:
    let
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
    in
    {
      devShell = forAllSystems
        (system:
          let
            pkgs = import nixpkgs {
              inherit system;
            };
          in
            pkgs.mkShell {
              # required for bindgen
              nativeBuildInputs  = with pkgs; [ clang_11 cmake ];

              # required for bindgen
              LIBCLANG_PATH = "${pkgs.llvmPackages_11.libclang.lib}/lib";
            }
        );
    };
}
