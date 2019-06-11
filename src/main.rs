mod targets;

use clap::{App, Arg};
use failure::Error;
use jsl::{Schema, SerdeSchema};
use std::fs::File;
use std::io::{BufWriter, Write};

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

        // let out_writer: Box<Write> = match out {
        //     "-" => Box::new(std::io::stdout()),
        //     _ => Box::new(File::create(out)?),
        // };

        // targets::typescript::render(&mut BufWriter::new(out_writer), &schema)?;
    }

    if let Some(out) = matches.value_of("java-out") {
        println!("Output to java: {}", out);
    }

    Ok(())
}
