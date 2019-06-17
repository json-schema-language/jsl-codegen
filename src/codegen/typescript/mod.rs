use crate::codegen;
use failure::{format_err, Error};
use inflector::Inflector;
use jsl::schema::{Form, Type};
use jsl::Schema;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Codegen {
    root_name: String,
    out_path: PathBuf,
}

impl codegen::Codegen for Codegen {
    type Ast = Vec<Ast>;

    fn args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
        app.arg(
            clap::Arg::with_name("ts-out")
                .help("TypeScript output directory")
                .takes_value(true)
                .long("ts-out"),
        )
    }

    fn from_args(matches: &clap::ArgMatches) -> Result<Option<Codegen>, Error> {
        if let Some(ts_out) = matches.value_of("ts-out") {
            let input = PathBuf::from(matches.value_of("INPUT").unwrap());
            let input_stem = input
                .file_stem()
                .ok_or(format_err!("Could not infer file stem from input"))?;
            let input_stem_str = input_stem
                .to_str()
                .ok_or(format_err!("Could not convert input file name to UTF-8"))?;
            let root_name = input_stem_str.to_pascal_case();
            let out_file_name = format!("{}.ts", root_name);

            Ok(Some(Codegen {
                root_name,
                out_path: PathBuf::from(ts_out).join(out_file_name),
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
            let ast = self.transform_subschema(&mut out, &mut path, def);
            self.transform_for_id(&mut out, &path, ast);
        }

        // Then, generate the root schema.
        let mut path = vec![self.root_name.clone()];
        let ast = self.transform_subschema(&mut out, &mut path, schema);
        self.transform_for_id(&mut out, &path, ast);

        Ok(out)
    }

    fn serialize(&self, asts: &Self::Ast) -> Result<(), Error> {
        let mut out = BufWriter::new(File::create(self.out_path.clone())?);
        for ast in asts {
            self.serialize_subschema(&mut out, ast)?;
        }

        Ok(())
    }
}

impl Codegen {
    fn transform_subschema(
        &self,
        out: &mut Vec<Ast>,
        path: &mut Vec<String>,
        schema: &Schema,
    ) -> Ast {
        match schema.form() {
            Form::Empty => Ast::Any,
            Form::Ref(ref def) => Ast::Identifier(Self::path_to_identifier(&[def.clone()])),
            Form::Type(ref typ) => match typ {
                Type::Boolean => Ast::Boolean,
                Type::Number => Ast::Number,
                Type::String => Ast::String,
                Type::Timestamp => Ast::String,
            },
            Form::Enum(ref vals) => {
                Ast::Union(vals.iter().map(|v| Ast::Literal(v.clone())).collect())
            }
            Form::Elements(ref sub_schema) => {
                Ast::Array(Box::new(self.transform_subschema(out, path, sub_schema)))
            }
            Form::Properties(ref required, ref optional, _) => {
                let mut props = Vec::new();
                for (name, prop) in required {
                    props.push(self.transform_prop(out, path, true, name, prop));
                }

                for (name, prop) in optional {
                    props.push(self.transform_prop(out, path, false, name, prop));
                }

                let id = Self::path_to_identifier(path);
                out.push(Ast::Interface(id.clone(), props));
                Ast::Identifier(id)
            }
            Form::Values(ref sub_schema) => {
                Ast::Map(Box::new(self.transform_subschema(out, path, sub_schema)))
            }
            Form::Discriminator(ref tag, ref mapping) => {
                let mut cases = Vec::new();
                for (name, case) in mapping {
                    path.push(name.clone());

                    let mut props = Vec::new();
                    props.push(Property {
                        name: tag.clone(),
                        required: true,
                        value: Ast::Literal(name.clone()),
                    });

                    let (required, optional) = match case.form() {
                        Form::Properties(ref required, ref optional, _) => (required, optional),
                        _ => unreachable!("non-prop form in mapping"),
                    };

                    for (name, prop) in required {
                        props.push(self.transform_prop(out, path, true, name, prop));
                    }

                    for (name, prop) in optional {
                        props.push(self.transform_prop(out, path, false, name, prop));
                    }

                    let id = Self::path_to_identifier(path);
                    out.push(Ast::Interface(id.clone(), props));
                    cases.push(Ast::Identifier(id));

                    path.pop();
                }

                Ast::Union(cases)
            }
        }
    }

    fn transform_prop(
        &self,
        out: &mut Vec<Ast>,
        path: &mut Vec<String>,
        required: bool,
        name: &str,
        prop: &Schema,
    ) -> Property {
        path.push(name.to_owned());
        let value = self.transform_subschema(out, path, prop);
        path.pop();

        Property {
            name: name.to_owned(),
            required,
            value,
        }
    }

    // Ensure that an AST will get a top-level identifier, and then return an
    // AST for an identifier that refers to it.
    fn transform_for_id(&self, out: &mut Vec<Ast>, path: &Vec<String>, ast: Ast) -> Ast {
        match ast {
            Ast::Interface(_, _) | Ast::Type(_, _) | Ast::Identifier(_) => {}
            _ => {
                out.push(Ast::Type(Self::path_to_identifier(path), Box::new(ast)));
            }
        };

        Ast::Identifier(Self::path_to_identifier(path))
    }

    fn serialize_subschema(&self, out: &mut Write, ast: &Ast) -> Result<(), Error> {
        match ast {
            Ast::Any => write!(out, "any")?,
            Ast::Boolean => write!(out, "boolean")?,
            Ast::Number => write!(out, "number")?,
            Ast::String => write!(out, "string")?,
            Ast::Identifier(ref id) => write!(out, "{}", id)?,
            Ast::Literal(ref lit) => write!(out, "{:?}", lit)?,
            Ast::Array(ref ast) => {
                self.serialize_subschema(out, ast)?;
                write!(out, "[]")?;
            }
            Ast::Map(ref ast) => {
                write!(out, "{{ [name: string]: ")?;
                self.serialize_subschema(out, ast)?;
                write!(out, " }}")?;
            }
            Ast::Type(ref id, ref ast) => {
                write!(out, "export type {} = ", id)?;
                self.serialize_subschema(out, ast)?;
                writeln!(out, ";")?;
            }
            Ast::Interface(ref name, ref props) => {
                writeln!(out, "export interface {} {{", name)?;
                for prop in props {
                    let q_mark = if prop.required { "" } else { "?" };
                    write!(out, "  {}{}: ", prop.name, q_mark)?;
                    self.serialize_subschema(out, &prop.value)?;
                    writeln!(out, ";")?;
                }
                writeln!(out, "}}")?;
            }
            Ast::Union(ref asts) => {
                for ast in &asts[..asts.len() - 1] {
                    self.serialize_subschema(out, ast)?;
                    write!(out, " | ")?;
                }

                self.serialize_subschema(out, asts.last().unwrap())?;
            }
        };

        Ok(())
    }

    fn path_to_identifier(path: &[String]) -> String {
        path.join("_").to_pascal_case()
    }
}

#[derive(Debug)]
pub enum Ast {
    Any,
    Boolean,
    Number,
    String,

    // An identifier.
    Identifier(String),

    // A constant string type.
    Literal(String),

    // An array with elements of some type.
    Array(Box<Ast>),

    // A map from strings to some type.
    Map(Box<Ast>),

    // A type declaration.
    Type(String, Box<Ast>),

    // An interface with a name and properties.
    Interface(String, Vec<Property>),

    // A union of types.
    Union(Vec<Ast>),
}

#[derive(Debug)]
pub struct Property {
    name: String,
    required: bool,
    value: Ast,
}
