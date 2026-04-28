use clap::{Parser, Subcommand};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ===================================================
//  code-stats - Source Code Analysis Tool
//  Built with Rust | Developed with Claude AI
// ===================================================

const DEFAULT_IGNORE: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".idea",
    ".vscode",
    "dist",
    "build",
    "__pycache__",
    ".next",
    "vendor",
];

// --------------------------------------------------
//  Data Structures
// --------------------------------------------------

#[derive(Debug)]
struct FileStats {
    #[allow(dead_code)]
    path: PathBuf,
    total_lines: usize,
    code_lines: usize,
    blank_lines: usize,
    comment_lines: usize,
    extension: String,
}

#[derive(Debug, Default)]
struct ExtensionSummary {
    file_count: usize,
    total_lines: usize,
    code_lines: usize,
}

// --------------------------------------------------
//  CLI setup via clap
// --------------------------------------------------

#[derive(Parser)]
#[command(
    name = "code-stats",
    about = "Source code analyzer - get full stats about your project",
    version = "0.1.0",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to the project (default: current directory)
    #[arg(default_value = ".")]
    path: String,

    /// File extensions to ignore (e.g. --ignore-ext txt,log)
    #[arg(long, value_delimiter = ',')]
    ignore_ext: Vec<String>,

    /// Extra directories to ignore
    #[arg(long, value_delimiter = ',')]
    ignore_dir: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show a full tree map of the project (folders & files)
    Tree {
        #[arg(long)]
        files_only: bool,
    },
    /// Show line counts broken down by file type
    Lines,
    /// Quick overview: files, folders, lines, code ratio
    Stats,
    /// Show everything at once (default)
    All,
}

// --------------------------------------------------
//  Analysis logic
// --------------------------------------------------

fn should_ignore(path: &Path, extra_ignore: &[String]) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        DEFAULT_IGNORE.contains(&name.as_ref())
            || extra_ignore.iter().any(|i| i == name.as_ref())
    })
}

fn analyze_file(path: &Path) -> Option<FileStats> {
    let content = fs::read_to_string(path).ok()?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "no-ext".into())
        .to_string();

    let mut total   = 0usize;
    let mut blank   = 0usize;
    let mut comment = 0usize;
    let mut code    = 0usize;

    let comment_prefix = match ext.as_str() {
        "py" | "sh" | "rb" | "yml" | "yaml" | "toml" => "#",
        "rs" | "js" | "ts" | "c" | "cpp" | "h" | "java" | "go" | "swift" => "//",
        "html" | "xml" => "<!--",
        "css" | "scss" => "/*",
        _ => "//",
    };

    for line in content.lines() {
        total += 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank += 1;
        } else if trimmed.starts_with(comment_prefix) {
            comment += 1;
        } else {
            code += 1;
        }
    }

    Some(FileStats {
        path: path.to_path_buf(),
        total_lines: total,
        code_lines: code,
        blank_lines: blank,
        comment_lines: comment,
        extension: ext,
    })
}

fn collect_files(root: &Path, ignore_ext: &[String], ignore_dir: &[String]) -> Vec<FileStats> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                return !should_ignore(e.path(), ignore_dir);
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            if ignore_ext.is_empty() {
                return true;
            }
            let ext = e
                .path()
                .extension()
                .map(|x| x.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            !ignore_ext.iter().any(|i| i.to_lowercase() == ext.as_ref())
        })
        .filter(|e| !should_ignore(e.path(), ignore_dir))
        .filter_map(|e| analyze_file(e.path()))
        .collect()
}

// --------------------------------------------------
//  Language name mapping
// --------------------------------------------------

fn ext_to_language(ext: &str) -> &'static str {
    match ext {
        "rs"                    => "Rust",
        "py"                    => "Python",
        "js"                    => "JavaScript",
        "ts"                    => "TypeScript",
        "go"                    => "Go",
        "c"                     => "C",
        "cpp" | "cc" | "cxx"    => "C++",
        "h" | "hpp"             => "C/C++ Header",
        "java"                  => "Java",
        "kt" | "kts"            => "Kotlin",
        "swift"                 => "Swift",
        "rb"                    => "Ruby",
        "php"                   => "PHP",
        "cs"                    => "C#",
        "html" | "htm"          => "HTML",
        "css"                   => "CSS",
        "scss" | "sass"         => "SCSS/Sass",
        "sh" | "bash"           => "Shell",
        "sql"                   => "SQL",
        "json"                  => "JSON",
        "yaml" | "yml"          => "YAML",
        "toml"                  => "TOML",
        "md" | "markdown"       => "Markdown",
        "xml"                   => "XML",
        "lua"                   => "Lua",
        "dart"                  => "Dart",
        "r"                     => "R",
        "ex" | "exs"            => "Elixir",
        "hs"                    => "Haskell",
        "lock"                  => "Lockfile",
        _                       => "Other",
    }
}

// --------------------------------------------------
//  Tree display
// --------------------------------------------------

fn print_tree(root: &Path, ignore_dir: &[String]) {
    println!("{}", format!("  {}/", root.display()).bright_cyan().bold());
    print_tree_recursive(root, root, "", ignore_dir);
}

fn print_tree_recursive(root: &Path, current: &Path, prefix: &str, ignore_dir: &[String]) {
    let mut entries: Vec<_> = match fs::read_dir(current) {
        Ok(r) => r.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    // Dirs first, then files, alphabetically
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        b_is_dir.cmp(&a_is_dir).then(a.file_name().cmp(&b.file_name()))
    });

    let entries: Vec<_> = entries
        .into_iter()
        .filter(|e| !should_ignore(&e.path(), ignore_dir))
        .collect();

    let count = entries.len();

    for (i, entry) in entries.iter().enumerate() {
        let is_last      = i == count - 1;
        let connector    = if is_last { "+--> " } else { "|--- " };
        let continuation = if is_last { "     " } else { "|    " };

        let path   = entry.path();
        let name   = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if is_dir {
            println!(
                "{}{}{}",
                prefix,
                connector,
                format!("[DIR] {}/", name).bright_cyan()
            );
            print_tree_recursive(
                root,
                &path,
                &format!("{}{}", prefix, continuation),
                ignore_dir,
            );
        } else {
            let tag          = file_tag(&name);
            let colored_name = color_file_name(&name);
            println!("{}{}{} {}", prefix, connector, tag, colored_name);
        }
    }
}

fn file_tag(name: &str) -> ColoredString {
    let ext = name.split('.').last().unwrap_or("").to_lowercase();
    let label = match ext.as_str() {
        "rs"                           => "[rs] ",
        "py"                           => "[py] ",
        "js"                           => "[js] ",
        "ts"                           => "[ts] ",
        "go"                           => "[go] ",
        "c" | "cpp" | "h"             => "[c]  ",
        "java"                         => "[jv] ",
        "html" | "htm"                 => "[htm]",
        "css" | "scss"                 => "[css]",
        "json"                         => "[jsn]",
        "yaml" | "toml" | "yml"        => "[cfg]",
        "md"                           => "[md] ",
        "sh" | "bash"                  => "[sh] ",
        "sql"                          => "[sql]",
        "txt"                          => "[txt]",
        "png" | "jpg" | "jpeg"
        | "gif" | "svg"                => "[img]",
        _                              => "[---]",
    };
    label.bright_black()
}

fn color_file_name(name: &str) -> ColoredString {
    let ext = name.split('.').last().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs"                 => name.bright_red().bold(),
        "py"                 => name.yellow().bold(),
        "js" | "ts"          => name.bright_yellow(),
        "go"                 => name.cyan(),
        "c" | "cpp" | "h"   => name.blue(),
        "java"               => name.bright_red(),
        "html" | "htm"       => name.bright_magenta(),
        "css" | "scss"       => name.magenta(),
        "json" | "yaml"
        | "toml" | "yml"     => name.bright_green(),
        "md"                 => name.white().bold(),
        "sh" | "bash"        => name.green(),
        _                    => name.white(),
    }
}

// --------------------------------------------------
//  Stats display
// --------------------------------------------------

fn sep(width: usize) {
    println!("{}", "-".repeat(width).bright_black());
}

fn print_lines_stats(files: &[FileStats]) {
    let mut by_ext: HashMap<&str, ExtensionSummary> = HashMap::new();

    for f in files {
        let s = by_ext.entry(f.extension.as_str()).or_default();
        s.file_count  += 1;
        s.total_lines += f.total_lines;
        s.code_lines  += f.code_lines;
    }

    let mut sorted: Vec<_> = by_ext.iter().collect();
    sorted.sort_by(|a, b| b.1.code_lines.cmp(&a.1.code_lines));

    let total_files: usize = sorted.iter().map(|(_, s)| s.file_count).sum();
    let total_code:  usize = sorted.iter().map(|(_, s)| s.code_lines).sum();
    let total_lines: usize = sorted.iter().map(|(_, s)| s.total_lines).sum();

    println!("\n{}", "=".repeat(82).bright_blue());
    println!("{}", "  Lines Breakdown by Language".bright_white().bold());
    println!("{}", "=".repeat(82).bright_blue());

    println!(
        "  {:<10} {:<18} {:>6} {:>10} {:>10}  {:<22}",
        "Ext".bright_cyan().bold(),
        "Language".bright_cyan().bold(),
        "Files".bright_cyan().bold(),
        "Code".bright_cyan().bold(),
        "Total".bright_cyan().bold(),
        "Share".bright_cyan().bold(),
    );
    sep(82);

    for (ext, summary) in &sorted {
        let lang = ext_to_language(ext);

        let pct = if total_code > 0 {
            summary.code_lines * 100 / total_code
        } else {
            0
        };

        // Progress bar (max 18 chars wide)
        let bar_len = if total_code > 0 {
            (summary.code_lines * 18 / total_code).max(if summary.code_lines > 0 { 1 } else { 0 })
        } else {
            0
        };
        let bar   = "#".repeat(bar_len);
        let empty = ".".repeat(18usize.saturating_sub(bar_len));

        // Dominance label
        let dominance = if pct >= 60 {
            "Dominant  "
        } else if pct >= 30 {
            "Major     "
        } else if pct >= 10 {
            "Significant"
        } else if pct >= 1 {
            "Minor     "
        } else {
            "Negligible"
        };

        println!(
            "  {:<10} {:<18} {:>6} {:>10} {:>10}  {:>3}% [{}{}] {}",
            format!(".{}", ext).yellow(),
            lang.white(),
            summary.file_count.to_string().white(),
            summary.code_lines.to_string().bright_green().bold(),
            summary.total_lines.to_string().white(),
            pct,
            bar.bright_blue(),
            empty.bright_black(),
            dominance.bright_yellow(),
        );
    }

    sep(82);
    println!(
        "  {:<10} {:<18} {:>6} {:>10} {:>10}",
        "TOTAL".bright_white().bold(),
        "",
        total_files.to_string().bright_white().bold(),
        total_code.to_string().bright_green().bold(),
        total_lines.to_string().bright_white().bold(),
    );
    println!("{}\n", "=".repeat(82).bright_blue());
}

fn print_full_stats(root: &Path, files: &[FileStats]) {
    let dir_count = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.path() != root)
        .filter(|e| !should_ignore(e.path(), &[]))
        .count();

    let total_lines:   usize = files.iter().map(|f| f.total_lines).sum();
    let total_code:    usize = files.iter().map(|f| f.code_lines).sum();
    let total_blank:   usize = files.iter().map(|f| f.blank_lines).sum();
    let total_comment: usize = files.iter().map(|f| f.comment_lines).sum();

    println!("\n{}", "=".repeat(50).bright_blue());
    println!("{}", "  Project Overview".bright_white().bold());
    println!("{}", "=".repeat(50).bright_blue());

    let row = |label: &str, value: String| {
        println!(
            "  {:<24} {}",
            label.bright_cyan(),
            value.bright_yellow().bold()
        );
    };

    row("Directories",   dir_count.to_string());
    row("Total Files",   files.len().to_string());
    row("Total Lines",   format_number(total_lines));
    row("Code Lines",    format_number(total_code));
    row("Comment Lines", format_number(total_comment));
    row("Blank Lines",   format_number(total_blank));

    if total_lines > 0 {
        let pct = total_code * 100 / total_lines;
        row("Code Ratio", format!("{}%", pct));
    }

    println!("{}\n", "=".repeat(50).bright_blue());
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

// --------------------------------------------------
//  Entry point
// --------------------------------------------------

fn main() {
    let cli  = Cli::parse();
    let root = PathBuf::from(&cli.path);

    if !root.exists() {
        eprintln!("{}", "  ERROR: path not found.".bright_red().bold());
        std::process::exit(1);
    }

    // Banner
    println!();
    println!("{}", "   ██████╗ ██████╗ ██████╗ ███████╗    ███████╗████████╗ █████╗ ████████╗███████╗".bright_blue());
    println!("{}", "  ██╔════╝██╔═══██╗██╔══██╗██╔════╝    ██╔════╝╚══██╔══╝██╔══██╗╚══██╔══╝██╔════╝".bright_blue());
    println!("{}", "  ██║     ██║   ██║██║  ██║█████╗      ███████╗   ██║   ███████║   ██║   ███████╗".bright_cyan());
    println!("{}", "  ██║     ██║   ██║██║  ██║██╔══╝      ╚════██║   ██║   ██╔══██║   ██║   ╚════██║".bright_cyan());
    println!("{}", "  ╚██████╗╚██████╔╝██████╔╝███████╗    ███████║   ██║   ██║  ██║   ██║   ███████║".cyan());
    println!("{}", "   ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝    ╚══════╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝   ╚══════╝".cyan());
    println!("  {}", "Source Code Analyzer  |  Built with Rust  |  Developed with Claude AI".bright_black());
    println!();

    let command = cli.command.unwrap_or(Commands::All);

    match command {
        Commands::Tree { files_only: _ } => {
            println!("{}", "=".repeat(60).bright_blue());
            println!("{}", "  Project Tree".bright_white().bold());
            println!("{}", "=".repeat(60).bright_blue());
            print_tree(&root, &cli.ignore_dir);
        }
        Commands::Lines => {
            let files = collect_files(&root, &cli.ignore_ext, &cli.ignore_dir);
            print_lines_stats(&files);
        }
        Commands::Stats => {
            let files = collect_files(&root, &cli.ignore_ext, &cli.ignore_dir);
            print_full_stats(&root, &files);
        }
        Commands::All => {
            let files = collect_files(&root, &cli.ignore_ext, &cli.ignore_dir);
            print_full_stats(&root, &files);
            print_lines_stats(&files);
            println!("{}", "=".repeat(60).bright_blue());
            println!("{}", "  Project Tree".bright_white().bold());
            println!("{}", "=".repeat(60).bright_blue());
            print_tree(&root, &cli.ignore_dir);
        }
    }
}
