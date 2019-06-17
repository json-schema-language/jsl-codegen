use crate::codegen;
use failure::{format_err, Error};
use inflector::Inflector;
use jsl::schema::{Form, Type};
use jsl::Schema;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Codegen {
    root_name: String,
    out_dir: PathBuf,
    out_pkg: Vec<String>,
}

impl codegen::Codegen for Codegen {
    type Ast = Vec<TopLevel>;

    fn args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
        app.args(&[
            clap::Arg::with_name("java-out")
                .help("Java output directory")
                .takes_value(true)
                .long("java-out"),
            clap::Arg::with_name("java-pkg")
                .help("Java output package")
                .takes_value(true)
                .long("java-pkg"),
        ])
    }

    fn from_args(matches: &clap::ArgMatches) -> Result<Option<Codegen>, Error> {
        if let Some(java_out) = matches.value_of("java-out") {
            let input = PathBuf::from(matches.value_of("INPUT").unwrap());
            let input_stem = input
                .file_stem()
                .ok_or(format_err!("Could not infer file stem from input"))?;
            let input_stem_str = input_stem
                .to_str()
                .ok_or(format_err!("Could not convert input file name to UTF-8"))?;
            let root_name = input_stem_str.to_class_case();

            let java_pkg: Vec<_> = matches
                .value_of("java-pkg")
                .ok_or(format_err!("--java-pkg required for Java output"))?
                .split(".")
                .map(|s| s.to_owned())
                .collect();

            Ok(Some(Codegen {
                root_name,
                out_dir: PathBuf::from(java_out).join(java_pkg.join("/")),
                out_pkg: java_pkg,
            }))
        } else {
            Ok(None)
        }
    }

    fn transform(&self, schema: &Schema) -> Result<Self::Ast, Error> {
        let mut out = Vec::new();

        // First, generate each of the definitions.
        for (name, def) in schema.definitions().as_ref().unwrap() {
            let mut path = vec![name.clone()];
            self.transform_subschema(&mut out, &mut path, def);
        }

        // Then, generate the root schema.
        let mut path = vec![self.root_name.clone()];
        self.transform_subschema(&mut out, &mut path, schema);

        Ok(out)
    }

    fn serialize(&self, ast: &Self::Ast) -> Result<(), Error> {
        for top_level in ast {
            let name = match top_level {
                TopLevel::Class(ref name, _) => name,
                TopLevel::Enum(ref name, _) => name,
            };

            let path = self.out_dir.join(format!("{}.java", name));
            let mut out = BufWriter::new(File::create(path.clone())?);

            writeln!(out, "package {};", self.out_pkg.join("."))?;
            writeln!(out, "")?;

            match top_level {
                TopLevel::Class(ref name, ref props) => {
                    writeln!(out, "public class {} {{", name)?;
                    for (name, value) in props {
                        writeln!(out, "    public {} {};", value.unboxed(), name)?;
                    }
                    writeln!(out, "}}")?;
                }
                TopLevel::Enum(ref name, ref vals) => {
                    writeln!(out, "public enum {} {{", name)?;
                    for val in vals {
                        writeln!(out, "   {},", val)?;
                    }
                    writeln!(out, "}}")?;
                }
            };
        }

        Ok(())
    }
}

impl Codegen {
    fn transform_subschema(
        &self,
        out: &mut Vec<TopLevel>,
        path: &mut Vec<String>,
        schema: &Schema,
    ) -> JavaType {
        match schema.form() {
            Form::Empty => JavaType::Object,
            Form::Ref(ref def) => JavaType::Identifer(Self::path_to_identifier(&[def.to_owned()])),
            Form::Type(Type::Boolean) => JavaType::Boolean,
            Form::Type(Type::Number) => JavaType::Double,
            Form::Type(Type::String) => JavaType::String,
            Form::Type(Type::Timestamp) => JavaType::Timestamp,
            Form::Enum(ref vals) => {
                let name = Self::path_to_identifier(&path);
                out.push(TopLevel::Enum(name.clone(), vals.iter().cloned().collect()));
                JavaType::Identifer(name)
            }
            Form::Elements(ref sub_schema) => {
                JavaType::List(Box::new(self.transform_subschema(out, path, sub_schema)))
            }
            Form::Properties(ref required, ref optional, _) => {
                let mut props = HashMap::new();
                for (name, prop) in required {
                    path.push(name.clone());
                    let value = self.transform_subschema(out, path, prop);
                    path.pop();

                    props.insert(name.clone(), value);
                }

                for (name, prop) in optional {
                    path.push(name.clone());
                    let value = self.transform_subschema(out, path, prop);
                    path.pop();

                    props.insert(name.clone(), value);
                }

                let name = Self::path_to_identifier(&path);
                out.push(TopLevel::Class(name.clone(), props));
                JavaType::Identifer(name)
            }
            Form::Values(ref sub_schema) => {
                JavaType::Map(Box::new(self.transform_subschema(out, path, sub_schema)))
            }
            Form::Discriminator(_, _) => {
                JavaType::Object // TODO
            }
        }
    }

    fn path_to_identifier(path: &[String]) -> String {
        path.join("_").to_class_case()
    }
}

#[derive(Debug)]
pub enum TopLevel {
    Class(String, HashMap<String, JavaType>),
    Enum(String, Vec<String>),
}

#[derive(Debug)]
pub enum JavaType {
    Object,
    Boolean,
    Double,
    String,
    Timestamp,
    Identifer(String),
    List(Box<JavaType>),
    Map(Box<JavaType>),
}

impl JavaType {
    fn unboxed(&self) -> String {
        match self {
            JavaType::Object => "Object".to_owned(),
            JavaType::Boolean => "boolean".to_owned(),
            JavaType::Double => "double".to_owned(),
            JavaType::String => "String".to_owned(),
            JavaType::Timestamp => "Instant".to_owned(),
            JavaType::Identifer(ref id) => id.to_owned(),
            JavaType::List(ref typ) => format!("List<{}>", typ.boxed()),
            JavaType::Map(ref typ) => format!("Map<String, {}>", typ.boxed()),
        }
    }

    fn boxed(&self) -> String {
        match self {
            JavaType::Object => "Object".to_owned(),
            JavaType::Boolean => "Boolean".to_owned(),
            JavaType::Double => "Double".to_owned(),
            JavaType::String => "String".to_owned(),
            JavaType::Timestamp => "Instant".to_owned(),
            JavaType::Identifer(ref id) => id.to_owned(),
            JavaType::List(ref typ) => format!("List<{}>", typ.boxed()),
            JavaType::Map(ref typ) => format!("Map<String, {}>", typ.boxed()),
        }
    }
}
