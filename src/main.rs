mod targets;

use clap::{App, Arg};
use failure::Error;
use jsl::{Schema, SerdeSchema};
use std::fs::File;
use failure::format_err;

fn main() -> Result<(), Error> {
    let matches = App::new("JSON Schema Language Codegen")
        .version("1.0")
        .about("Generates code from a JSON Schema Language schema.")
        .arg(
            Arg::with_name("INPUT")
                .help("Input JSON Schema Language schema")
                .last(true)
                .required(true),
        )
        .arg(
            Arg::with_name("ts-out")
                .help("TypeScript output directory.")
                .takes_value(true)
                .long("ts-out"),
        )
        .arg(
            Arg::with_name("ts-file")
                .help("Force a TypeScript file name, rather than inferring.")
                .takes_value(true)
                .long("ts-file"),
        )
        .arg(
            Arg::with_name("java-out")
                .help("Java output directory.")
                .takes_value(true)
                .long("java-out"),
        )
        .arg(
            Arg::with_name("java-pkg")
                .help("Java output package.")
                .takes_value(true)
                .long("java-pkg"),
        )
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let file = File::open(input)?;
    let serde_schema: SerdeSchema = serde_json::from_reader(file)?;
    let schema = Schema::from_serde(serde_schema)?;

    if let Some(out) = matches.value_of("ts-out") {
        let config = targets::typescript::Config {
            out_dir: out.into(),
            in_file: input.into(),
            out_file: matches.value_of("ts-file").map(|s| s.to_owned()),
        };

        targets::typescript::codegen(&config, &schema)?;
    }

    if let Some(out) = matches.value_of("java-out") {
        let out_pkg = matches.value_of("java-pkg").ok_or(format_err!("--java-pkg is required when java-out is set"))?;

        let config = targets::java::Config {
            out_dir: out.into(),
            in_file: input.into(),
            out_pkg: out_pkg.to_owned(),
        };

        targets::java::codegen(&config, &schema)?;
    }

    Ok(())
}
