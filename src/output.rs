use colored::Colorize;

use crate::cli::Source;
use crate::model::Package;

pub fn print_packages(
    repo_packages: &[Package],
    aur_packages: &[Package],
    source: Source,
    bottom_up: bool,
    json: bool,
    no_color: bool,
) -> anyhow::Result<()> {
    if json {
        let mut all = Vec::new();
        if matches!(source, Source::Repo | Source::All) {
            all.extend_from_slice(repo_packages);
        }
        if matches!(source, Source::Aur | Source::All) {
            all.extend_from_slice(aur_packages);
        }
        println!("{}", serde_json::to_string_pretty(&all)?);
        return Ok(());
    }

    if no_color {
        colored::control::set_override(false);
    }

    // Order handling
    let mut sections: Vec<(&str, &[Package])> = vec![];
    if bottom_up {
        // AUR first
        if matches!(source, Source::Aur | Source::All) {
            sections.push(("aur", aur_packages));
        }
        if matches!(source, Source::Repo | Source::All) {
            sections.push(("repo", repo_packages));
        }
    } else {
        // Repo first (default like pacman)
        if matches!(source, Source::Repo | Source::All) {
            sections.push(("repo", repo_packages));
        }
        if matches!(source, Source::Aur | Source::All) {
            sections.push(("aur", aur_packages));
        }
    }

    let mut total = 0usize;
    for (_kind, pkgs) in &sections {
        total += pkgs.len();
    }

    if total == 0 {
        eprintln!("{}", "No packages found.".yellow());
        return Ok(());
    }

    for (kind, pkgs) in sections {
        for pkg in pkgs {
            print_package(pkg, kind);
        }
    }

    eprintln!(
        "\n{} {} (repo: {}, aur: {})",
        "Found".dimmed(),
        total.to_string().bold(),
        repo_packages.len(),
        aur_packages.len()
    );

    Ok(())
}

fn print_package(pkg: &Package, kind: &str) {
    // Format: repo/name version [installed] (votes/pop) - desc
    let repo_part = format!("{}/{}", pkg.repo, pkg.name);
    let repo_colored = if kind == "aur" {
        repo_part.bright_magenta().bold()
    } else {
        match pkg.repo.as_str() {
            "core" => repo_part.red().bold(),
            "extra" => repo_part.green().bold(),
            "multilib" => repo_part.blue().bold(),
            "system" | "world" | "galaxy" | "lib32" => repo_part.cyan().bold(),
            _ => repo_part.yellow().bold(),
        }
    };

    let version = pkg.version.white();
    let installed = if pkg.installed {
        " [installed]".bright_green().bold().to_string()
    } else {
        "".to_string()
    };

    let aur_extra = if pkg.repo == "aur" {
        let votes = pkg.votes.unwrap_or(0);
        let pop = pkg.popularity.unwrap_or(0.0);
        let ood = pkg
            .out_of_date
            .map(|_| " [out-of-date]".red().to_string())
            .unwrap_or_default();
        format!(" (+{} {:.2}){}", votes, pop, ood)
    } else {
        "".to_string()
    };

    let desc = pkg.short_desc().dimmed();

    println!("{} {} {}{}", repo_colored, version, aur_extra, installed);
    println!("    {}", desc);
}

pub fn print_json_error(msg: &str) {
    let v = serde_json::json!({"error": msg});
    let s =
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| format!(r#"{{"error":"{}"}}"#, msg));
    eprintln!("{}", s);
}
