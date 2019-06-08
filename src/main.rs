mod targets;

use clap::{App, Arg};
use failure::Error;
use jsl::{Schema, SerdeSchema};
use std::fs::File;

fn main() -> Result<(), Error> {
    let matches = App::new("JSON Schema Language Codegen")
        .version("1.0")
        .about("Generates code from a JSON Schema Language schema.")
        .arg(
            Arg::with_name("INPUT")
                .help("Input schema(s)")
                .last(true)
                .required(true),
        )
        .arg(
            Arg::with_name("ts-out")
                .help("TypeScript output location")
                .long("ts-out"),
        )
        .get_matches();

    let input = matches.value_of("INPUT").unwrap();
    let file = File::open(input)?;
    let serde_schema: SerdeSchema = serde_json::from_reader(file)?;
    let schema = Schema::from_serde(serde_schema)?;

    targets::typescript::render(&mut std::io::stdout(), &schema)?;

    Ok(())
}
