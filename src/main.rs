use anyhow::Result;
use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg};
use difference::{Changeset, Difference};
use regex::Regex;
use std::io::Write;
use std::process::{Command, Stdio};

fn main() -> Result<()> {
    let matches = App::new("gf")
        .setting(AppSettings::DisableHelpSubcommand)
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .arg(
            Arg::with_name("input")
                .help("the input text to generate furigana")
                .index(1)
                .required(true)
                .short("i")
                .long("input")
                .value_name("TEXT")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("bold")
                .help("the text to highlight")
                .index(2)
                .short("b")
                .long("bold")
                .value_name("TEXT")
                .takes_value(true),
        )
        .get_matches();

    let input = matches.value_of("input").expect("should never fail");
    let jumanpp_output = get_jumanpp_output(input)?;
    if let Some(highlight) = matches.value_of("bold") {
        println!("{}", parse_jumanpp_output(&jumanpp_output, &highlight)?);
    } else {
        println!("{}", parse_jumanpp_output(&jumanpp_output, "")?);
    }

    Ok(())
}

fn generate_furigana(kanji: &str, yomi: &str) -> Result<String> {
    let mut text = String::new();

    let changeset = Changeset::new(kanji, yomi, "");
    for (i, _x) in changeset.diffs.iter().enumerate() {
        if let Difference::Rem(kanji) = &changeset.diffs[i] {
            if let Difference::Add(furigana) = &changeset.diffs[i + 1] {
                text += format!("<ruby><rb>{}<rt>{}</ruby>", kanji, furigana).as_str();
            } else if let Difference::Same(furigana) = &changeset.diffs[i + 1] {
                text += format!("<ruby><rb>{}<rt>{}</ruby>", kanji, furigana).as_str();
            }
        } else if let Difference::Same(same) = &changeset.diffs[i] {
            text += same;
        }
    }
    Ok(text)
}

fn get_jumanpp_output(input: &str) -> Result<String> {
    let mut child = Command::new("jumanpp")
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let child_stdin = child
        .stdin
        .as_mut()
        .expect("Child process stdin has not been captured!");
    child_stdin.write_all(input.as_bytes())?;
    // Close stdin to finish and avoid indefinite blocking
    drop(child_stdin);

    let output = child.wait_with_output()?;

    if output.status.success() {
        let raw_output = String::from_utf8(output.stdout)?;
        Ok(raw_output)
    } else {
        let err = String::from_utf8(output.stderr)?;
        panic!("External command failed:\n {}", err);
    }
}

fn parse_jumanpp_output(jumanpp_output: &str, highlight: &str) -> Result<String> {
    let re_ignore = Regex::new(r"^@").unwrap(); // ignore line starts with @
    let mut ret = String::new();
    for x in jumanpp_output.lines() {
        if x == "EOS" {
            break;
        } else if !re_ignore.is_match(x) {
            let v: Vec<&str> = x.split(" ").collect();

            let kanji_count = v[0]
                .chars()
                .filter(kanji::is_kanji)
                .collect::<Vec<char>>()
                .len();

            let tmp = generate_furigana(&v[0], &v[1])?;

            if kanji_count > 0 {
                ret += tmp.as_str();
            } else {
                ret += &v[0];
            }
        }
    }

    if highlight != "" {
        let re_bold_furigana =
            Regex::new(format!("(?P<kanji>{})", regex::escape(&highlight)).as_str())?;
        let bold_sentence_furigana = re_bold_furigana.replace_all(&ret, "<b>$kanji</b>");
        return Ok(bold_sentence_furigana.to_string());
    } else {
        Ok(ret)
    }
}
