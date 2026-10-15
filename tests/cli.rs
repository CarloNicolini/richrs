use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
fn run(args: &[&str], input: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rich"))
        .args(args)
        .env("COLUMNS", "40")
        .env("TERM", "dumb")
        .env_remove("RICH_THEME")
        .env_remove("NO_COLOR")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}
fn output(args: &[&str], input: &str) -> String {
    let o = run(args, input);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
fn temp(ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "richrs-test-{}-{}.{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed),
        ext
    ))
}
#[test]
fn cli_help_version_and_head_alias() {
    assert!(output(&["--help"], "").contains("--head"));
    assert!(output(&["-v"], "").contains("0.1.0"));
    assert_eq!(output(&["-", "-h", "1"], "one\ntwo\n"), "one\n");
}
#[test]
fn markup_plain_and_forced_color() {
    assert_eq!(output(&["-p", "[bold red]Hello[/]"], ""), "Hello\n");
    assert!(output(&["-p", "[red]Hello[/]", "--force-terminal"], "").contains("\x1b["));
}
#[test]
fn emoji_opt_in() {
    assert_eq!(output(&["-p", ":rocket:", "-j"], ""), "🚀\n");
    assert_eq!(output(&["-p", ":rocket:"], ""), ":rocket:\n");
}
#[test]
fn wrapping_and_alignment() {
    assert_eq!(
        output(&["-p", "one two three", "-w", "7"], ""),
        "one two\nthree\n"
    );
    assert_eq!(output(&["-p", "hi", "-w", "6", "-R"], ""), "    hi\n");
    assert_eq!(output(&["-p", "hi", "--soft", "-w", "1"], ""), "hi\n");
}
#[test]
fn syntax_range_and_guides() {
    let o = output(
        &["-", "-x", "python", "-n", "-g", "-t", "1"],
        "x = 1\n    print(x)\n",
    );
    assert!(o.contains("2 │   print(x)"));
    assert!(!o.contains("x = 1"));
}
#[test]
fn json_preserves_key_order() {
    let o = output(&["-", "-J"], "{\"z\":1,\"a\":[true,null]}");
    assert!(o.find("\"z\"").unwrap() < o.find("\"a\"").unwrap());
    assert!(o.contains("true"));
}
#[test]
fn invalid_inputs_fail() {
    for (args, data) in [
        (vec!["-", "-J"], "{"),
        (vec!["-p", "[/red]"], ""),
        (vec!["-p", "hi", "-d", "1,2,3"], ""),
        (vec!["-", "-h", "0"], "hi"),
        (vec!["-", "-h", "1", "-t", "1"], "hi"),
        (vec!["-p", "hi", "-s", "garbage"], ""),
        (vec!["-p", "hi", "-w", "0"], ""),
        (vec!["--inspect", "42"], ""),
    ] {
        assert!(!run(&args, data).status.success(), "{args:?}");
    }
}
#[test]
fn all_panel_types_titles_captions_padding() {
    for kind in ["ascii", "ascii2", "square", "rounded", "heavy", "double"] {
        let o = output(
            &[
                "-p",
                "hello",
                "-a",
                kind,
                "--title",
                "Title",
                "--caption",
                "Foot",
                "-d",
                "1,2",
                "-w",
                "24",
            ],
            "",
        );
        assert!(o.contains("Title"));
        assert!(o.contains("Foot"));
        assert!(o.contains("hello"));
        assert!(
            o.lines()
                .all(|l| unicode_width::UnicodeWidthStr::width(l) <= 24)
        );
    }
}
#[test]
fn rules() {
    let o = output(&["-u", "Test", "--rule-char", "=", "-w", "20"], "");
    assert_eq!(o.trim_end().chars().count(), 20);
    assert!(o.contains(" Test "));
    assert!(!run(&["-u", "hi", "--rule-char", ""], "").status.success());
}
#[test]
fn markdown_structures() {
    let o = output(
        &["-", "-m"],
        "# Heading\n\n- First\n- **Second**\n\n> Quote\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|---|\n| x | 1 |\n",
    );
    for s in ["Heading", "• First", "Second", "│ Quote", "fn main()", "x"] {
        assert!(o.contains(s), "missing {s}: {o}");
    }
}
#[test]
fn csv_quoted_delimiters_and_tail() {
    let o = output(
        &["-", "--csv", "--tail", "1"],
        "name,score\n\"hello, world\",1\nlast,20\n",
    );
    assert!(o.contains("name"));
    assert!(o.contains("last"));
    assert!(!o.contains("hello"));
}
#[test]
fn notebook_cells_outputs() {
    let o = output(
        &["-", "--ipynb"],
        r##"{"metadata":{"kernelspec":{"language":"python"}},"cells":[{"cell_type":"markdown","source":["# Title"]},{"cell_type":"code","execution_count":1,"source":["print(42)"],"outputs":[{"output_type":"stream","text":["42\n"]},{"output_type":"execute_result","execution_count":1,"data":{"text/plain":["42"]}},{"output_type":"error","traceback":["Error"]}]}]}"##,
    );
    for s in ["Title", "In [1]:", "print(42)", "Out[1]:", "Error"] {
        assert!(o.contains(s), "{o}");
    }
}
#[test]
fn rst_common_and_unsupported() {
    let o = output(
        &["-", "--rst"],
        "Title\n=====\n\n.. code-block:: python\n\n   print(42)\n",
    );
    assert!(o.contains("Title"));
    assert!(o.contains("print(42)"));
    assert!(
        !run(&["-", "--rst"], ".. include:: /etc/passwd")
            .status
            .success()
    );
}
#[test]
fn exports_preserve_styles_and_escape() {
    let h = temp("html");
    let s = temp("svg");
    output(
        &[
            "-p",
            "[red]<hello & world>[/]",
            "-o",
            h.to_str().unwrap(),
            "--export-svg",
            s.to_str().unwrap(),
        ],
        "",
    );
    let html = std::fs::read_to_string(&h).unwrap();
    let svg = std::fs::read_to_string(&s).unwrap();
    assert!(html.contains("color:#800000"));
    assert!(html.contains("&lt;hello &amp; world&gt;"));
    assert!(svg.contains("<svg"));
    assert!(svg.contains("&lt;hello &amp; world&gt;"));
    std::fs::remove_file(h).unwrap();
    std::fs::remove_file(s).unwrap();
}
#[test]
fn auto_detection_and_lossy_utf8() {
    let p = temp("json");
    std::fs::write(&p, "{\"ok\":true}").unwrap();
    assert!(output(&[p.to_str().unwrap()], "").contains("\"ok\""));
    std::fs::remove_file(p).unwrap();
    let p = temp("txt");
    std::fs::write(&p, [b'a', 255]).unwrap();
    assert!(output(&[p.to_str().unwrap()], "").contains('�'));
    std::fs::remove_file(p).unwrap();
}
#[test]
fn narrow_unicode_panels_do_not_panic() {
    for w in ["1", "2", "3", "4", "5"] {
        output(&["-p", "界e\u{301}😀", "-a", "rounded", "-w", w], "");
    }
}
#[test]
fn nonterminal_pager_falls_back_to_stdout() {
    assert_eq!(output(&["-p", "hello", "--pager"], ""), "hello\n");
}

#[test]
fn quoted_commas_do_not_override_semicolon_delimiter() {
    let out = output(
        &["-", "--csv"],
        "name;description\none;\"a,b,c,d,e\"\ntwo;\"f,g,h,i,j\"\n",
    );
    assert!(out.contains("description"));
    assert!(out.contains("a,b,c,d,e"));
}
#[test]
fn bundled_theme_palettes_and_named_colors() {
    for theme in ["dracula", "monokai", "default", "friendly"] {
        assert!(
            output(
                &["-", "-x", "python", "--theme", theme, "--force-terminal"],
                "print(42)"
            )
            .contains("\x1b[")
        );
    }
    assert!(output(&["-p", "[dark_orange3]hi[/]", "--force-terminal"], "").contains("\x1b["));
}

#[test]
fn format_flag_precedence_matches_upstream() {
    assert_eq!(output(&["-p", "hello", "--syntax"], ""), "hello\n");
    assert_eq!(output(&["-", "--syntax", "--rule"], "hello"), "hello\n");
    assert!(output(&["-", "--json", "--csv"], "{\"ok\":true}").contains("\"ok\""));
}
