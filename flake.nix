{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    self,
    flake-utils,
    naersk,
    nixpkgs,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = (import nixpkgs) {
          inherit system;
        };

        naersk' =
          pkgs.callPackage naersk {
          };

        editor = pkgs.buildNpmPackage {
          pname = "editor";
          version = "1.0.0";

          src = ./editor;

          npmDepsHash = "sha256-NfOpXLoau0In/kTo2jJjgrgFqTOakNgjcPleFJ+YnpU=";

          installPhase = "cp -r dist $out";
        };

        server = naersk'.buildPackage {
          nativeBuildInputs = with pkgs; [pkg-config rustPlatform.bindgenHook];
          buildInputs = with pkgs; [openssl sqlite];
          src = ./.;
        };

        link_start = pkgs.writeShellScriptBin "start" ''
          mkdir editor
          rm editor/dist
          ln -s ${editor} editor/dist
          ${server}/bin/advent-of-wasm
        '';
      in {
        # For `nix build` & `nix run`:
        defaultPackage = server;

        nixosModules.default = {...}: {
          systemd.services.advent-of-wasm = {
            wantedBy = ["multi-user.target"];
            serviceConfig = {
              ExecStart = "${link_start}/bin/start";
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
          nativeBuildInputs = with pkgs; [rustc cargo];
        };
      }
    );
}
