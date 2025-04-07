{
  pkgs,
  config,
  lib,
  ...
}:
with lib; let
  cfg = config.services.redirector;

  settingsFormat = pkgs.formats.toml {};
  settingsFile = settingsFormat.generate "redirector-config.toml" cfg.settings;
in {
  options.services.redirector = {
    enable = mkEnableOption "Redirector, a simple URL redirector";

    package = mkOption {
      type = types.package;
      default = pkgs.redirector;
      description = "The redirector package to use.";
    };

    settings = mkOption {
      type = settingsFormat.type;
      default = {};
      description = "Configuration for redirector.";
      example = literalExpression ''
        {
          port = 3000;
          bangs_url = "https://duckduckgo.com/bang.js";
          default_search = "https://www.qwant.com/?q={}";
          bangs = [
            {
              trigger = "gh";
              url_template = "https://github.com/search?q={{{s}}}";
              short_name = "GitHub";
              category = "Tech";
            }
          ];
        }
      '';
    };

    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port to listen on.";
    };

    ip = mkOption {
      type = types.str;
      default = "0.0.0.0";
      description = "IP address to bind to.";
    };

    defaultSearch = mkOption {
      type = types.str;
      default = "https://www.qwant.com/?q={}";
      description = "Default search engine URL template (use '{}' as placeholder for the query).";
    };

    bangsUrl = mkOption {
      type = types.str;
      default = "https://duckduckgo.com/bang.js";
      description = "URL to fetch bang commands from.";
    };

    customBangs = mkOption {
      type = types.listOf (
        types.submodule {
          options = {
            trigger = mkOption {
              type = types.str;
              description = "The trigger text for the bang command.";
              example = "gh";
            };

            url_template = mkOption {
              type = types.str;
              description = "The URL template where the search term is inserted.";
              example = "https://github.com/search?q={{{s}}}";
            };

            short_name = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "The short name or abbreviation of the bang command.";
              example = "GitHub";
            };

            category = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "The category of the bang command.";
              example = "Tech";
            };

            subcategory = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "The subcategory of the bang command.";
              example = "Programming";
            };

            domain = mkOption {
              type = types.nullOr types.str;
              default = null;
              description = "The domain associated with the bang command.";
              example = "github.com";
            };

            relevance = mkOption {
              type = types.nullOr types.int;
              default = null;
              description = "The relevance score of the bang command.";
              example = 10;
            };
          };
        }
      );
      default = [];
      description = "Custom bang commands.";
    };
  };

  config = mkIf cfg.enable {
    # Create the configuration directory
    home.file.".config/redirector/config.toml".source = settingsFile;

    # Merge all settings
    services.redirector.settings = mkMerge [
      {
        port = cfg.port;
        ip = cfg.ip;
        default_search = cfg.defaultSearch;
        bangs_url = cfg.bangsUrl;
      }
      (mkIf (cfg.customBangs != []) {
        bangs = map (bang: filterAttrs (name: value: value != null) bang) cfg.customBangs;
      })
    ];

    # Install the package
    home.packages = [cfg.package];

    # Add systemd user service
    systemd.user.services.redirector = {
      Unit = {
        Description = "Redirector search service";
        After = ["network.target"];
      };

      Service = {
        ExecStart = "${getExe cfg.package}";
        Restart = "on-failure";
      };

      Install = {
        WantedBy = ["default.target"];
      };
    };
  };
}
