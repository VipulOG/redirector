{
  pkgs,
  config,
  lib,
  ...
}:
with lib; let
  cfg = config.services.redirector;
  configFormat = pkgs.formats.toml {};
  configFile = configFormat.generate "config.toml" cfg.settings;
in {
  options.services.redirector = {
    enable = mkEnableOption "Redirector, a simple URL redirector";

    package = mkOption {
      type = types.package;
      default = pkgs.redirector;
      description = "The redirector package to use.";
    };

    settings = mkOption {
      type = configFormat.type;
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
  };

  config = mkIf cfg.enable {
    home.file.".config/redirector/config.toml".source = configFile;
    home.packages = [cfg.package];

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
