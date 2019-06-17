mod codegen;

use crate::codegen::Codegen;
use clap::{App, Arg};
use failure::Error;
use jsl::{Schema, SerdeSchema};
use std::fs::File;

fn main() -> Result<(), Error> {
    let app = App::new("jsl-codegen")
        .version("1.0")
        .about("Generates code from a JSON Schema Language schema")
        .arg(
            Arg::with_name("INPUT")
                .help("Input JSON Schema Language schema file")
                .last(true)
                .required(true),
        );

    // Set up the CLI for each of the code generators.
    let app = codegen::typescript::Codegen::args(app);
    let app = codegen::java::Codegen::args(app);
    let app = codegen::go::Codegen::args(app);

    // Parse out the input args.
    let matches = app.get_matches();

    // Prepare the code generators from the input args.
    let ts_codegen = codegen::typescript::Codegen::from_args(&matches)?;
    let java_codegen = codegen::java::Codegen::from_args(&matches)?;
    let go_codegen = codegen::go::Codegen::from_args(&matches)?;

    // Parse out the input schema, and ensure it is valid.
    let input = matches.value_of("INPUT").unwrap();
    let file = File::open(input)?;
    let serde_schema: SerdeSchema = serde_json::from_reader(file)?;
    let schema = Schema::from_serde(serde_schema)?;

    // Run each of the code generator transformation routines. If any fail, do
    // not generate code.
    let ts_ast = if let Some(ref cg) = ts_codegen {
        Some(cg.transform(&schema)?)
    } else {
        None
    };
    let java_ast = if let Some(ref cg) = java_codegen {
        Some(cg.transform(&schema)?)
    } else {
        None
    };
    let go_ast = if let Some(ref cg) = go_codegen {
        Some(cg.transform(&schema)?)
    } else {
        None
    };

    // Serialize each of the ASTs. At this point, only IO errors can cause
    // issues. That's sort of an inevitable state of affairs.
    if let Some(ref cg) = ts_codegen {
        cg.serialize(&ts_ast.unwrap())?;
    }
    if let Some(ref cg) = java_codegen {
        cg.serialize(&java_ast.unwrap())?;
    }
    if let Some(ref cg) = go_codegen {
        cg.serialize(&go_ast.unwrap())?;
    }

    Ok(())
}
