use failure::{format_err, Error};
use inflector::Inflector;
use jsl::schema::{Form, Type};
use jsl::Schema;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

pub struct Config {
    pub out_dir: String,
    pub in_file: String,
    pub out_file: Option<String>,
}

pub fn codegen(config: &Config, schema: &Schema) -> Result<(), Error> {
    // The strategy for generating TypeScript code is as follows:
    //
    // All code is generated into a single .ts file, whose name is inferred from
    // config.InFile and is appended to OutDir, unless a FileName is explictly
    // set.
    //
    // Each definition is generated as a type with the same name as the
    // definition, except with the case changed to be TypeScript-idiomatic. Then
    // we generate the root schema. Only properties and discriminator forms get
    // their own types, everything else is merely embedded directly into the
    // parent data structure.
    let in_file = PathBuf::from(&config.in_file);
    let in_file_stem = in_file.file_stem().ok_or(format_err!(
        "Could not determine file stem from input file."
    ))?;
    let in_file_stem = in_file_stem
        .to_str()
        .ok_or(format_err!("Could not convert file name to string."))?;
    let inferred_file_out = format!("{}.ts", in_file_stem);
    let file_name_out = config.out_file.as_ref().unwrap_or(&inferred_file_out);

    let out_path = PathBuf::from(&config.out_dir).join(file_name_out);

    let mut out = BufWriter::new(File::create(out_path)?);
    render(&mut out, in_file_stem, schema)?;

    Ok(())
}

fn render(out: &mut Write, root_name: &str, schema: &Schema) -> Result<(), Error> {
    for (name, sub_schema) in schema.definitions().as_ref().unwrap() {
        let mut path = vec![name.clone()];
        let name = render_schema(out, &mut path, sub_schema)?;

        match sub_schema.form() {
            Form::Empty
            | Form::Ref(_)
            | Form::Type(_)
            | Form::Elements(_)
            | Form::Enum(_)
            | Form::Values(_) => {
                // These forms don't write out any types, but the "ref" form
                // codegen presumes that definitions will always produce
                // eponymous types.
                //
                // To handle this, we create a type alias here.
                writeln!(out, "export type {} = {};", path_to_identifier(&path), name)?;
            }
            _ => {}
        }
    }

    let mut path = vec![root_name.to_owned()];
    let root = render_schema(out, &mut path, schema)?;
    writeln!(out, "export default {};", root)?;

    Ok(())
}

fn render_schema(
    out: &mut Write,
    path: &mut Vec<String>,
    schema: &Schema,
) -> Result<String, Error> {
    match schema.form() {
        Form::Empty => Ok("any".to_owned()),
        Form::Ref(ref def) => Ok(path_to_identifier(&mut vec![def.to_owned()])),
        Form::Type(typ) => match typ {
            Type::Boolean => Ok("boolean".to_owned()),
            Type::Number => Ok("number".to_owned()),
            Type::String | Type::Timestamp => Ok("string".to_owned()),
        },
        Form::Elements(ref sub_schema) => {
            let sub_name = render_schema(out, path, sub_schema)?;
            Ok(format!("{}[]", sub_name))
        }
        Form::Enum(ref values) => {
            let values: Vec<_> = values.iter().map(|v| format!("\"{}\"", v)).collect();
            Ok(values.join(" | "))
        }
        Form::Properties(ref required, ref optional, _) => {
            if let Some(comment) = schema.extra().get("description").and_then(|d| d.as_str()) {
                for line in comment.split("\n") {
                    writeln!(out, "// {}", line)?;
                }
            }
            render_interface(out, path, None, required, optional)
        }
        Form::Values(ref sub_schema) => {
            let sub_name = render_schema(out, path, sub_schema)?;
            Ok(format!("{{ [name: string]: {} }}", sub_name))
        }
        Form::Discriminator(ref tag, ref mapping) => {
            let mut sub_names = Vec::new();
            for (name, sub_schema) in mapping {
                path.push(name.clone());

                match sub_schema.form() {
                    Form::Properties(ref required, ref optional, _) => {
                        sub_names.push(render_interface(
                            out,
                            path,
                            Some((tag.clone(), name.clone())),
                            required,
                            optional,
                        )?);
                    }
                    _ => unreachable!("child of discriminator is not of properties form"),
                }

                path.pop();
            }

            let name = path_to_identifier(path);
            writeln!(out, "export type {} = {};", name, sub_names.join(" | "))?;
            Ok(name)
        }
    }
}

// Utility function which can be shared by properties and discriminator forms,
// which both want to generate TypeScript interfaces.
fn render_interface(
    out: &mut Write,
    path: &mut Vec<String>,
    tag: Option<(String, String)>,
    required: &HashMap<String, Schema>,
    optional: &HashMap<String, Schema>,
) -> Result<String, Error> {
    let mut required_props = HashMap::new();
    if let Some((tag_name, tag_value)) = tag {
        required_props.insert(tag_name, (format!("\"{}\"", tag_value), None));
    }

    for (name, sub_schema) in required {
        path.push(name.clone());
        let sub_name = render_schema(out, path, sub_schema)?;
        path.pop();

        let comment = sub_schema
            .extra()
            .get("description")
            .and_then(|d| d.as_str());
        required_props.insert(name.clone(), (sub_name, comment));
    }

    let mut optional_props = HashMap::new();
    for (name, sub_schema) in optional {
        path.push(name.clone());
        let sub_name = render_schema(out, path, sub_schema)?;
        path.pop();

        let comment = sub_schema
            .extra()
            .get("description")
            .and_then(|d| d.as_str());
        optional_props.insert(name.clone(), (sub_name, comment));
    }

    let name = path_to_identifier(path);
    writeln!(out, "export interface {} {{", name)?;

    for (name, (prop, comment)) in required_props {
        if let Some(comment) = comment {
            for line in comment.split("\n") {
                writeln!(out, "  // {}", line)?;
            }
        }

        writeln!(out, "  {}: {};", name, prop)?;
    }

    for (name, (prop, comment)) in optional_props {
        if let Some(comment) = comment {
            for line in comment.split("\n") {
                writeln!(out, "  // {}", line)?;
            }
        }

        writeln!(out, "  {}?: {};", name, prop)?;
    }

    writeln!(out, "}}")?;
    Ok(name.to_owned())
}

fn path_to_identifier(path: &Vec<String>) -> String {
    path.join("_").to_class_case()
}
