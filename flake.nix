{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, flake-utils, naersk, nixpkgs }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk' = pkgs.callPackage naersk {};

      in rec {
        # For `nix build` & `nix run`:
        defaultPackage = naersk'.buildPackage {
          nativeBuildInputs = with pkgs; [ pkg-config rustPlatform.bindgenHook ];
          buildInputs = with pkgs; [ openssl sqlite ];
          src = ./.;
        };

        nixosModules.default = { ... }: {
          systemd.services.advent-of-wasm = {
            wantedBy = [ "multi-user.target" ];
            serviceConfig = {
              ExecStart = "${defaultPackage}/bin/advent-of-wasm";
              User = "advent-of-wasm";
              Group = "advent-of-wasm";
              WorkingDirectory = "/var/lib/advent-of-wasm";
              StateDirectory = "advent-of-wasm";
            };
          };
          
          users.users.advent-of-wasm = {
            isSystemUser = true;
            group = "advent-of-wasm";
          };
          users.groups.advent-of-wasm = {};

          services.nginx = {
            recommendedProxySettings = true;

            virtualHosts."wasm.lucasholten.com" = {
              enableACME = true;
              forceSSL = true;

              locations."/" = {
                proxyPass = "http://127.0.0.1:3000";
              };
            };
          };
        };

        # For `nix develop`:
        devShell = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ];
        };
      }
    );
}
