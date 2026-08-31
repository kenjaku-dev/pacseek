use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "pacseek",
    version,
    about = "Fast search for Arch/Artix official repos + AUR",
    long_about = "Search both official pacman repositories (via libalpm) and the AUR (via RPC v5) in one command.\nExample: pacseek firefox --limit 20 --json",
    arg_required_else_help = true
)]
pub struct Cli {
    /// Search query (substring, case-insensitive). Use --regex for regex mode.
    pub query: String,

    /// Where to search
    #[arg(short, long, value_enum, default_value_t = Source::All, help = "Source to search")]
    pub source: Source,

    /// AUR search field (only affects AUR)
    #[arg(long, value_enum, default_value_t = AurBy::NameDesc, help = "AUR search by field")]
    pub by: AurBy,

    /// Limit results per source (0 = no limit)
    #[arg(short, long, default_value_t = 50)]
    pub limit: usize,

    /// Show results as JSON
    #[arg(long)]
    pub json: bool,

    /// Use regex matching (client-side filter for AUR, libalpm regex for repo)
    #[arg(short = 'x', long)]
    pub regex: bool,

    /// Show only installed packages (repo only)
    #[arg(long)]
    pub installed_only: bool,

    /// Print search results bottom-up (AUR first)
    #[arg(long)]
    pub bottom_up: bool,

    /// Verbose logs (tracing)
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Disable colored output
    #[arg(long)]
    pub no_color: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum Source {
    Aur,
    Repo,
    All,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum AurBy {
    Name,
    #[value(name = "name-desc")]
    NameDesc,
    Maintainer,
    Depends,
    Makedepends,
    Optdepends,
    Checkdepends,
}

impl AurBy {
    pub fn as_str(&self) -> &'static str {
        match self {
            AurBy::Name => "name",
            AurBy::NameDesc => "name-desc",
            AurBy::Maintainer => "maintainer",
            AurBy::Depends => "depends",
            AurBy::Makedepends => "makedepends",
            AurBy::Optdepends => "optdepends",
            AurBy::Checkdepends => "checkdepends",
        }
    }
}
