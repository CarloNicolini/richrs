use clap::{Parser, ValueHint};

/// Native terminal formatting: files, URLs, stdin, and console markup.
#[derive(Parser, Debug)]
#[command(
    name = "rich",
    version,
    disable_help_flag = true,
    disable_version_flag = true
)]
pub struct Args {
    /// Path, URL, literal markup (--print/--rule), or '-' for stdin
    #[arg(default_value = "", value_hint = ValueHint::FilePath)]
    pub resource: String,
    #[arg(long, action = clap::ArgAction::Help)]
    /// Show command help.
    pub help: Option<bool>,
    #[arg(short = 'v', long, action = clap::ArgAction::Version)]
    /// Print the richrs version.
    pub version: Option<bool>,
    #[arg(short = 'p', long = "print")]
    /// Print literal Rich console markup.
    pub print: bool,
    #[arg(short = 'u', long)]
    /// Draw a horizontal rule with an optional markup label.
    pub rule: bool,
    #[arg(short = 'J', long)]
    /// Validate, pretty-print and highlight JSON.
    pub json: bool,
    #[arg(short = 'm', long)]
    /// Render Markdown.
    pub markdown: bool,
    #[arg(long)]
    /// Render the supported reStructuredText subset.
    pub rst: bool,
    #[arg(long)]
    /// Render delimited data as a table.
    pub csv: bool,
    #[arg(long)]
    /// Render Jupyter notebook cells and text outputs.
    pub ipynb: bool,
    #[arg(long)]
    /// Force source-code highlighting.
    pub syntax: bool,
    /// Python object inspection (requires a Python runtime; unsupported)
    #[arg(long)]
    pub inspect: bool,
    #[arg(short = 'h', long, value_parser = clap::value_parser!(usize), conflicts_with = "tail")]
    /// Display the first N source lines or data rows.
    pub head: Option<usize>,
    #[arg(short = 't', long, value_parser = clap::value_parser!(usize))]
    /// Display the last N source lines or data rows.
    pub tail: Option<usize>,
    #[arg(short = 'j', long)]
    /// Expand Rich emoji shortcodes such as :rocket:.
    pub emoji: bool,
    #[arg(short = 'l', long)]
    /// Align the output block to the left.
    pub left: bool,
    #[arg(short = 'r', long)]
    /// Align the output block to the right.
    pub right: bool,
    #[arg(short = 'c', long)]
    /// Center the output block.
    pub center: bool,
    #[arg(short = 'L', long)]
    /// Left-justify text in print mode.
    pub text_left: bool,
    #[arg(short = 'R', long)]
    /// Right-justify text in print mode.
    pub text_right: bool,
    #[arg(short = 'C', long)]
    /// Center text in print mode.
    pub text_center: bool,
    #[arg(short = 'F', long)]
    /// Fully justify wrapped text in print mode.
    pub text_full: bool,
    #[arg(long)]
    /// Let the terminal wrap print output without inserting line breaks.
    pub soft: bool,
    #[arg(short = 'e', long)]
    /// Expand a panel or padded content to the available width.
    pub expand: bool,
    #[arg(short = 'w', long, value_parser = clap::value_parser!(u16).range(1..))]
    /// Render at this width in terminal cells.
    pub width: Option<u16>,
    #[arg(short = 'W', long, value_parser = clap::value_parser!(u16).range(1..))]
    /// Limit the rendering width in terminal cells.
    pub max_width: Option<u16>,
    #[arg(short = 's', long, default_value = "")]
    /// Apply a Rich text style, for example bold red on black.
    pub style: String,
    #[arg(long, default_value = "bright_green")]
    /// Style the horizontal rule.
    pub rule_style: String,
    #[arg(long, default_value = "─")]
    /// Repeat these characters to draw a rule.
    pub rule_char: String,
    #[arg(short = 'd', long)]
    /// Add padding: one, two or four comma-separated integers.
    pub padding: Option<String>,
    #[arg(short = 'a', long, default_value = "none", value_parser = ["none", "ascii", "ascii2", "square", "rounded", "heavy", "double"])]
    /// Surround content with a box.
    pub panel: String,
    #[arg(short = 'S', long, default_value = "")]
    /// Style the panel border.
    pub panel_style: String,
    #[arg(long, env = "RICH_THEME", default_value = "ansi_dark")]
    /// Select a bundled syntax theme; see docs/COMPATIBILITY.md.
    pub theme: String,
    #[arg(short = 'n', long)]
    /// Display source line numbers.
    pub line_numbers: bool,
    #[arg(short = 'g', long)]
    /// Display indentation guides.
    pub guides: bool,
    #[arg(short = 'x', long)]
    /// Select a syntax grammar, for example python or rust.
    pub lexer: Option<String>,
    #[arg(short = 'y', long)]
    /// Emit terminal hyperlinks instead of inline Markdown URLs.
    pub hyperlinks: bool,
    #[arg(long)]
    /// Crop source or print lines instead of wrapping them.
    pub no_wrap: bool,
    #[arg(long, default_value = "")]
    /// Set a panel or table title.
    pub title: String,
    #[arg(long, default_value = "")]
    /// Set a panel or table caption.
    pub caption: String,
    #[arg(long)]
    /// Emit ANSI styles even when stdout is redirected.
    pub force_terminal: bool,
    #[arg(short = 'o', long)]
    /// Save styled output as standalone HTML.
    pub export_html: Option<std::path::PathBuf>,
    #[arg(long)]
    /// Save styled output as standalone SVG.
    pub export_svg: Option<std::path::PathBuf>,
    #[arg(long)]
    /// Open the native pager (or PAGER when set).
    pub pager: bool,
    /// Generate shell argument completions and exit.
    #[arg(long, value_enum, hide = true)]
    pub generate_completions: Option<clap_complete::Shell>,
}

impl Args {
    pub fn padding(&self) -> Result<[usize; 4], String> {
        let Some(raw) = &self.padding else {
            return Ok([0; 4]);
        };
        let values: Vec<usize> = raw
            .split(',')
            .map(|s| s.trim().parse::<usize>())
            .collect::<Result<_, _>>()
            .map_err(|_| "padding must be 1, 2 or 4 nonnegative integers")?;
        if values.iter().any(|&n| n > 4096) {
            return Err("padding cannot exceed 4096 cells".into());
        }
        match values.as_slice() {
            [a] => Ok([*a; 4]),
            [a, b] => Ok([*a, *b, *a, *b]),
            [a, b, c, d] => Ok([*a, *b, *c, *d]),
            _ => Err("padding must be 1, 2 or 4 nonnegative integers".into()),
        }
    }
}
