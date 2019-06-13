use failure::{format_err, Error};
use inflector::Inflector;
use jsl::schema::{Form, Type};
use jsl::Schema;
use std::path::PathBuf;
use std::collections::HashMap;

pub struct Config {
    pub out_dir: String,
    pub out_pkg: String,
    pub in_file: String,
}

pub fn codegen(config: &Config, schema: &Schema) -> Result<(), Error> {
    let in_file = PathBuf::from(&config.in_file);
    let in_file_stem = in_file.file_stem().ok_or(format_err!(
        "Could not determine file stem from input file."
    ))?;
    let in_file_stem = in_file_stem
        .to_str()
        .ok_or(format_err!("Could not convert file name to string."))?;

    let mut def_names = HashMap::new();
    for (name, sub_schema) in schema.definitions().as_ref().unwrap() {
        let mut path = vec![name.clone()];
        let def_name = render_schema(&config.out_pkg, &def_names, &mut path, sub_schema)?;
        def_names.insert(name.clone(), def_name);
    }

    render_schema(
        &config.out_pkg,
        &def_names,
        &mut vec![in_file_stem.to_owned()],
        schema,
    )?;
    Ok(())
}

// Provides an expression which evaluates to a Java representation of a class.
//
// For the properties and discrminator forms, this will create a file containing
// the relevant class.
fn render_schema<'a>(
    pkg: &str,
    def_names: &HashMap<String, String>,
    path: &mut Vec<String>,
    schema: &Schema,
) -> Result<String, Error> {
    match schema.form() {
        Form::Empty => Ok("Object".to_owned()),
        Form::Ref(ref def) => Ok(def_names[def].clone()),
        Form::Type(ref typ) => Ok(match typ {
            Type::Boolean => "boolean",
            Type::Number => "double",
            Type::String => "String",
            Type::Timestamp => "Instant",
        }
        .to_owned()),
        Form::Properties(ref required, ref optional, _) => {
            let mut required_props = HashMap::new();
            for (name, sub_schema) in required {
                path.push(name.clone());
                let sub_name = render_schema(pkg, def_names, path, sub_schema)?;
                path.pop();

                required_props.insert(name, sub_name);
            }

            let mut optional_props = HashMap::new();
            for (name, sub_schema) in optional {
                path.push(name.clone());
                let sub_name = render_schema(pkg, def_names, path, sub_schema)?;
                path.pop();

                optional_props.insert(name, sub_name);
            }

            println!("public class {} {{", path_to_identifier(path));
            for (name, prop) in required_props {
                println!("    @NotNull");
                println!("    private {} {};", prop, name);
            }
            println!("}}");

            Ok("".to_owned())
        }
        _ => Ok("asdf".to_owned()),
    }
}

fn path_to_identifier(path: &Vec<String>) -> String {
    path.join("_").to_class_case()
}
