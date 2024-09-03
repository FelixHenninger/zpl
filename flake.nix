{
  description = "ZPL dev environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    systems.url = "github:nix-systems/default";
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      imports = [ inputs.treefmt-nix.flakeModule ];

      perSystem =
        {
          pkgs,
          system,
          config,
          lib,
          ...
        }:
        let
          craneLib = inputs.crane.mkLib pkgs;

          src =
            let
              spaFilter = path: _type: builtins.match ".*(html|css)$" path != null;
              filter = path: type: (spaFilter path type) || (craneLib.filterCargoSources path type);
            in
            lib.cleanSourceWith {
              src = ./.;
              inherit filter;
              name = "source";
            };

          commonArgs = {
            inherit src;
            strictDeps = true;
            buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [ pkgs.libiconv ];
          };

          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          individualCrateArgs = commonArgs // {
            inherit cargoArtifacts;
          };

          zpl = craneLib.buildPackage individualCrateArgs;

          server = craneLib.buildPackage (
            individualCrateArgs
            // {
              inherit (craneLib.crateNameFromCargoToml { src = ./server; }) pname version;
              cargoExtraArgs = "-p zpl-server";
              meta.mainProgram = "zpl-server";
            }
          );
        in
        {
          _module.args.pkgs =
            inputs.nixpkgs.legacyPackages.${system}.extend
              inputs.rust-overlay.overlays.default;

          packages = {
            default = zpl;
            inherit server;
          };

          devShells.default = craneLib.devShell {
            packages = [ config.treefmt.build.wrapper ];
            RUST_LOG = "debug";
          };

          treefmt = {
            projectRootFile = "flake.lock";

            settings.formatter = {
              nix = {
                command = pkgs.nixfmt-rfc-style;
                includes = [ "*.nix" ];
              };
              rustfmt = {
                command = pkgs.rustfmt;
                options = [
                  "--edition"
                  "2021"
                ];
                includes = [ "*.rs" ];
              };
            };
          };

          checks = lib.optionalAttrs pkgs.stdenv.isLinux {
            nixos-test = pkgs.nixosTest {
              name = "zpl-server-test";
              nodes.machine = {
                nixpkgs.system = system;
                imports = [ inputs.self.nixosModules.default ];
                services.zpl-server.enable = true;
              };
              testScript = ''
                machine.wait_for_unit("default.target")
                machine.wait_for_open_port(3000)
              '';
            };
          };
        };

      flake = {
        nixosModules.default =
          {
            config,
            pkgs,
            lib,
            ...
          }:
          let
            cfg = config.services.zpl-server;
            json = pkgs.formats.json { };
          in
          {
            options = {
              services.zpl-server = {
                enable = lib.mkEnableOption "Zebra printer service";
                package = lib.mkOption {
                  type = lib.types.package;
                  default = inputs.self.packages.${config.nixpkgs.system}.server;
                };
                listen = lib.mkOption {
                  type = lib.types.str;
                  default = "localhost:3000";
                };
                settings = lib.mkOption {
                  type = lib.types.attrsOf json.type;
                  default = { };
                };
              };
            };

            config = lib.mkIf cfg.enable {
              systemd.services.zpl-server = {
                description = "Zebra printer service";
                wantedBy = [ "multi-user.target" ];
                environment = {
                  ZPL_LISTEN = cfg.listen;
                  ZPL_CONFIGURATION = json.generate "zpl-server.json" cfg.settings;
                  RUST_LOG = "info";
                };
                serviceConfig = {
                  ExecStart = "${lib.getExe cfg.package}";
                  DynamicUser = true;
                  Restart = "always";
                  RestartSec = "2s";
                };
              };
            };
          };
      };
    };
}
