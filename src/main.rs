use clap::{Parser, Subcommand};
use colored::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

// ===================================================
//  code-stats — أداة تحليل المشاريع البرمجية
//  تم تطويرها بمساعدة الذكاء الاصطناعي (Claude)
// ===================================================

/// الدلائل التي يتم تجاهلها تلقائياً
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

// ──────────────────────────────────────────────────
//  هياكل البيانات
// ──────────────────────────────────────────────────

/// إحصائيات ملف واحد
#[derive(Debug)]
struct FileStats {
    path: PathBuf,
    total_lines: usize,
    code_lines: usize,
    blank_lines: usize,
    comment_lines: usize,
    extension: String,
}

/// ملخص نوع ملف معيّن
#[derive(Debug, Default)]
struct ExtensionSummary {
    file_count: usize,
    total_lines: usize,
    code_lines: usize,
}

// ──────────────────────────────────────────────────
//  إعداد CLI باستخدام clap
// ──────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "code-stats",
    about = "أداة تحليل الكود المصدري — تعطيك إحصائيات كاملة عن مشروعك",
    version = "0.1.0",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// المسار للمشروع (افتراضي: المجلد الحالي)
    #[arg(default_value = ".")]
    path: String,

    /// امتدادات ملفات يتم تجاهلها (مثال: --ignore-ext txt,log)
    #[arg(long, value_delimiter = ',')]
    ignore_ext: Vec<String>,

    /// دلائل إضافية لتجاهلها
    #[arg(long, value_delimiter = ',')]
    ignore_dir: Vec<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// عرض مخطط شجري كامل للمشروع
    Tree {
        /// إظهار الملفات فقط (بدون مجلدات فارغة)
        #[arg(long)]
        files_only: bool,
    },
    /// إحصائيات عدد الأسطر مع تفصيل لكل نوع ملف
    Lines,
    /// إحصائيات سريعة: عدد الملفات والأسطر والمجلدات
    Stats,
    /// عرض جميع المعلومات دفعة واحدة
    All,
}

// ──────────────────────────────────────────────────
//  منطق التحليل
// ──────────────────────────────────────────────────

/// يتحقق إذا كان المسار يجب تجاهله
fn should_ignore(path: &Path, extra_ignore: &[String]) -> bool {
    path.components().any(|c| {
        let name = c.as_os_str().to_string_lossy();
        DEFAULT_IGNORE.contains(&name.as_ref())
            || extra_ignore.iter().any(|i| i == name.as_ref())
    })
}

/// يقرأ ملفاً ويحسب أسطره بالتفصيل
fn analyze_file(path: &Path) -> Option<FileStats> {
    let content = fs::read_to_string(path).ok()?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_else(|| "بلا امتداد".into())
        .to_string();

    let mut total = 0usize;
    let mut blank = 0usize;
    let mut comment = 0usize;
    let mut code = 0usize;

    // رموز التعليقات الشائعة حسب الامتداد
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

/// يجمع كل ملفات المشروع مع تطبيق فلتر التجاهل
fn collect_files(root: &Path, ignore_ext: &[String], ignore_dir: &[String]) -> Vec<FileStats> {
    WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // تجاهل الدلائل المحددة
            if e.file_type().is_dir() {
                return !should_ignore(e.path(), ignore_dir);
            }
            true
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            // تجاهل الامتدادات المحددة من المستخدم
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

// ──────────────────────────────────────────────────
//  عرض المخطط الشجري
// ──────────────────────────────────────────────────

/// يرسم مخطط الشجرة بشكل جميل
fn print_tree(root: &Path, ignore_dir: &[String]) {
    println!("{}", format!("📁 {}", root.display()).bright_cyan().bold());
    print_tree_recursive(root, root, "", ignore_dir);
}

fn print_tree_recursive(root: &Path, current: &Path, prefix: &str, ignore_dir: &[String]) {
    // نقرأ محتوى المجلد ونرتبه: المجلدات أولاً ثم الملفات
    let mut entries: Vec<_> = match fs::read_dir(current) {
        Ok(r) => r.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        b_is_dir.cmp(&a_is_dir).then(a.file_name().cmp(&b.file_name()))
    });

    // نتجاهل الدلائل المحظورة
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|e| {
            let path = e.path();
            !should_ignore(&path, ignore_dir)
        })
        .collect();

    let count = entries.len();

    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let extension = if is_last { "    " } else { "│   " };

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if is_dir {
            println!(
                "{}{}{}",
                prefix,
                connector,
                format!("📁 {}/", name).bright_cyan()
            );
            print_tree_recursive(
                root,
                &path,
                &format!("{}{}", prefix, extension),
                ignore_dir,
            );
        } else {
            // نعطي لون مختلف لكل نوع ملف
            let icon = file_icon(&name);
            let colored_name = color_file_name(&name);
            println!("{}{}{} {}", prefix, connector, icon, colored_name);
        }
    }
}

/// أيقونة لكل نوع ملف
fn file_icon(name: &str) -> &'static str {
    let ext = name.split('.').last().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => "🦀",
        "py" => "🐍",
        "js" | "ts" => "⚡",
        "go" => "🐹",
        "c" | "cpp" | "h" => "⚙️ ",
        "java" => "☕",
        "html" | "htm" => "🌐",
        "css" | "scss" => "🎨",
        "json" | "yaml" | "toml" | "yml" => "📄",
        "md" => "📝",
        "sh" | "bash" => "🖥️ ",
        "sql" => "🗄️ ",
        "txt" => "📃",
        "png" | "jpg" | "jpeg" | "gif" | "svg" => "🖼️ ",
        _ => "📄",
    }
}

/// لون لكل نوع ملف
fn color_file_name(name: &str) -> ColoredString {
    let ext = name.split('.').last().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "rs" => name.bright_red().bold(),
        "py" => name.yellow().bold(),
        "js" | "ts" => name.bright_yellow(),
        "go" => name.cyan(),
        "c" | "cpp" | "h" => name.blue(),
        "java" => name.bright_red(),
        "html" | "htm" => name.bright_magenta(),
        "css" | "scss" => name.magenta(),
        "json" | "yaml" | "toml" | "yml" => name.bright_green(),
        "md" => name.white().bold(),
        "sh" | "bash" => name.green(),
        _ => name.white(),
    }
}

// ──────────────────────────────────────────────────
//  عرض الإحصائيات
// ──────────────────────────────────────────────────

fn print_separator(width: usize) {
    println!("{}", "─".repeat(width).bright_black());
}

fn print_lines_stats(files: &[FileStats]) {
    let mut by_ext: HashMap<&str, ExtensionSummary> = HashMap::new();

    for f in files {
        let s = by_ext.entry(f.extension.as_str()).or_default();
        s.file_count += 1;
        s.total_lines += f.total_lines;
        s.code_lines += f.code_lines;
    }

    // ترتيب حسب عدد أسطر الكود تنازلياً
    let mut sorted: Vec<_> = by_ext.iter().collect();
    sorted.sort_by(|a, b| b.1.code_lines.cmp(&a.1.code_lines));

    println!("\n{}", "━".repeat(60).bright_blue());
    println!(
        "{}",
        "  📊  إحصائيات الأسطر حسب نوع الملف"
            .bright_white()
            .bold()
    );
    println!("{}", "━".repeat(60).bright_blue());
    println!(
        "  {:<12} {:>8} {:>12} {:>10}",
        "الامتداد".bright_cyan().bold(),
        "ملفات".bright_cyan().bold(),
        "كود".bright_cyan().bold(),
        "إجمالي".bright_cyan().bold()
    );
    print_separator(60);

    let total_files: usize = sorted.iter().map(|(_, s)| s.file_count).sum();
    let total_code: usize = sorted.iter().map(|(_, s)| s.code_lines).sum();
    let total_lines: usize = sorted.iter().map(|(_, s)| s.total_lines).sum();

    for (ext, summary) in &sorted {
        let bar_len = if total_code > 0 {
            (summary.code_lines * 20 / total_code).max(if summary.code_lines > 0 { 1 } else { 0 })
        } else {
            0
        };
        let bar = "█".repeat(bar_len);
        println!(
            "  {:<12} {:>8} {:>12} {:>10}  {}",
            format!(".{}", ext).yellow(),
            summary.file_count.to_string().white(),
            summary.code_lines.to_string().bright_green().bold(),
            summary.total_lines.to_string().white(),
            bar.bright_blue()
        );
    }

    print_separator(60);
    println!(
        "  {:<12} {:>8} {:>12} {:>10}",
        "الإجمالي".bright_white().bold(),
        total_files.to_string().bright_white().bold(),
        total_code.to_string().bright_green().bold(),
        total_lines.to_string().bright_white().bold()
    );
    println!("{}\n", "━".repeat(60).bright_blue());
}

fn print_full_stats(root: &Path, files: &[FileStats]) {
    // حساب المجلدات
    let dir_count = WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.path() != root)
        .filter(|e| !should_ignore(e.path(), &[]))
        .count();

    let total_lines: usize = files.iter().map(|f| f.total_lines).sum();
    let total_code: usize = files.iter().map(|f| f.code_lines).sum();
    let total_blank: usize = files.iter().map(|f| f.blank_lines).sum();
    let total_comment: usize = files.iter().map(|f| f.comment_lines).sum();

    println!("\n{}", "━".repeat(50).bright_blue());
    println!("{}", "  🚀  نظرة عامة على المشروع".bright_white().bold());
    println!("{}", "━".repeat(50).bright_blue());

    let stat_line = |icon: &str, label: &str, value: String| {
        println!(
            "  {}  {:<20} {}",
            icon,
            label.bright_cyan(),
            value.bright_yellow().bold()
        );
    };

    stat_line("📁", "عدد المجلدات", dir_count.to_string());
    stat_line("📄", "عدد الملفات", files.len().to_string());
    stat_line("📏", "إجمالي الأسطر", format_number(total_lines));
    stat_line("💻", "أسطر الكود", format_number(total_code));
    stat_line("💬", "أسطر التعليقات", format_number(total_comment));
    stat_line("⬜", "أسطر فارغة", format_number(total_blank));

    // نسبة الكود
    if total_lines > 0 {
        let pct = total_code * 100 / total_lines;
        stat_line("📈", "نسبة الكود", format!("{}%", pct));
    }

    println!("{}\n", "━".repeat(50).bright_blue());
}

/// يضيف فواصل آلاف للأرقام الكبيرة
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

// ──────────────────────────────────────────────────
//  نقطة الدخول
// ──────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let root = PathBuf::from(&cli.path);

    if !root.exists() {
        eprintln!("{}", "❌  المسار المحدد غير موجود!".bright_red().bold());
        std::process::exit(1);
    }

    // شعار الأداة
    println!();
    println!("{}", "   ██████╗ ██████╗ ██████╗ ███████╗    ███████╗████████╗ █████╗ ████████╗███████╗".bright_blue());
    println!("{}", "  ██╔════╝██╔═══██╗██╔══██╗██╔════╝    ██╔════╝╚══██╔══╝██╔══██╗╚══██╔══╝██╔════╝".bright_blue());
    println!("{}", "  ██║     ██║   ██║██║  ██║█████╗      ███████╗   ██║   ███████║   ██║   ███████╗".bright_cyan());
    println!("{}", "  ██║     ██║   ██║██║  ██║██╔══╝      ╚════██║   ██║   ██╔══██║   ██║   ╚════██║".bright_cyan());
    println!("{}", "  ╚██████╗╚██████╔╝██████╔╝███████╗    ███████║   ██║   ██║  ██║   ██║   ███████║".cyan());
    println!("{}", "   ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝    ╚══════╝   ╚═╝   ╚═╝  ╚═╝   ╚═╝   ╚══════╝".cyan());
    println!(
        "  {}",
        "أداة تحليل الكود المصدري — تم تطويرها بمساعدة الذكاء الاصطناعي"
            .bright_black()
            .italic()
    );
    println!();

    let command = cli.command.unwrap_or(Commands::All);

    match command {
        Commands::Tree { files_only: _ } => {
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
            // نعرض كل شيء
            let files = collect_files(&root, &cli.ignore_ext, &cli.ignore_dir);
            print_full_stats(&root, &files);
            print_lines_stats(&files);
            println!("{}", "━".repeat(60).bright_blue());
            println!("{}", "  🌳  مخطط المشروع".bright_white().bold());
            println!("{}", "━".repeat(60).bright_blue());
            print_tree(&root, &cli.ignore_dir);
        }
    }
      }
      
