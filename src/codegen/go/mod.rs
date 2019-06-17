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
    out_path: PathBuf,
    out_pkg: String,
}

impl codegen::Codegen for Codegen {
    type Ast = Vec<Ast>;

    fn args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b> {
        app.arg(
            clap::Arg::with_name("go-out")
                .help("Go output directory")
                .takes_value(true)
                .long("go-out"),
        )
    }

    fn from_args(matches: &clap::ArgMatches) -> Result<Option<Codegen>, Error> {
        if let Some(go_out) = matches.value_of("go-out") {
            let input = PathBuf::from(matches.value_of("INPUT").unwrap());
            let input_stem = input
                .file_stem()
                .ok_or(format_err!("Could not infer file stem from input"))?;
            let input_stem_str = input_stem
                .to_str()
                .ok_or(format_err!("Could not convert input file name to UTF-8"))?;
            let root_name = input_stem_str.to_class_case();
            let pkg_name = go_out.split("/").last().unwrap();
            let out_file_name = format!("{}.go", pkg_name);

            Ok(Some(Codegen {
                root_name,
                out_path: PathBuf::from(go_out).join(out_file_name),
                out_pkg: pkg_name.to_owned(),
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
        writeln!(out, "package {}", self.out_pkg)?;

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
                Type::Timestamp => Ast::Time,
            },
            Form::Enum(ref vals) => {
                let name = Self::path_to_identifier(&path);
                out.push(Ast::Type(name.clone(), Box::new(Ast::String)));
                for val in vals {
                    out.push(Ast::Var(
                        (name.to_owned() + "_" + val).to_camel_case(),
                        Box::new(Ast::Identifier(name.to_owned())),
                        Box::new(Ast::Literal(val.to_owned())),
                    ));
                }

                Ast::Identifier(name)
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
                out.push(Ast::Struct(id.clone(), props, None));
                Ast::Identifier(id)
            }
            Form::Values(ref sub_schema) => {
                Ast::Map(Box::new(self.transform_subschema(out, path, sub_schema)))
            }
            Form::Discriminator(ref tag, ref mapping) => {
                let mut cases = Vec::new();
                let mut case_names = Vec::new();
                for (name, case) in mapping {
                    path.push(name.clone());

                    let mut props = Vec::new();
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
                    out.push(Ast::Struct(id.clone(), props, None));
                    cases.push(Ast::Identifier(id.clone()));
                    case_names.push(name.clone());

                    path.pop();
                }

                let mut mapping = HashMap::new();
                for (name, case) in case_names.into_iter().zip(cases) {
                    mapping.insert(name, case);
                }

                let id = Self::path_to_identifier(&path);
                let tag_name = tag.to_class_case();

                let props = vec![
                    Property {
                        name: tag_name,
                        json_name: tag.clone(),
                        value: Ast::String,
                    },
                    Property {
                        name: "Val".to_owned(),
                        json_name: "-".to_owned(),
                        value: Ast::Any,
                    },
                ];

                out.push(Ast::Struct(id.clone(), props, Some((tag.clone(), mapping))));
                Ast::Identifier(id)
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
            name: name.to_pascal_case(),
            json_name: name.to_owned(),
            value,
        }
    }

    // Ensure that an AST will get a top-level identifier, and then return an
    // AST for an identifier that refers to it.
    fn transform_for_id(&self, out: &mut Vec<Ast>, path: &Vec<String>, ast: Ast) -> Ast {
        match ast {
            Ast::Struct(_, _, _) | Ast::Type(_, _) | Ast::Identifier(_) => {}
            _ => {
                out.push(Ast::Type(Self::path_to_identifier(path), Box::new(ast)));
            }
        };

        Ast::Identifier(Self::path_to_identifier(path))
    }

    fn serialize_subschema(&self, out: &mut Write, ast: &Ast) -> Result<(), Error> {
        match ast {
            Ast::Any => write!(out, "interface{{}}")?,
            Ast::Boolean => write!(out, "bool")?,
            Ast::Number => write!(out, "float64")?,
            Ast::String => write!(out, "string")?,
            Ast::Time => write!(out, "time.Time")?,
            Ast::Identifier(ref id) => write!(out, "{}", id)?,
            Ast::Literal(ref lit) => write!(out, "{:?}", lit)?,
            Ast::Array(ref ast) => {
                write!(out, "[]")?;
                self.serialize_subschema(out, ast)?;
            }
            Ast::Map(ref ast) => {
                write!(out, "map[string]")?;
                self.serialize_subschema(out, ast)?;
            }
            Ast::Type(ref id, ref ast) => {
                write!(out, "type {} ", id)?;
                self.serialize_subschema(out, ast)?;
                writeln!(out)?;
            }
            Ast::Var(ref name, ref typ, ref val) => {
                write!(out, "var {} ", name)?;
                self.serialize_subschema(out, typ)?;
                write!(out, " = ")?;
                self.serialize_subschema(out, val)?;
                writeln!(out)?;
            }
            Ast::Struct(ref name, ref props, ref json) => {
                writeln!(out, "type {} struct {{", name)?;
                for prop in props {
                    write!(out, "\t{} ", prop.name)?;
                    self.serialize_subschema(out, &prop.value)?;
                    writeln!(out, " `json:\"{},omitempty\"`", prop.json_name)?;
                }
                writeln!(out, "}}")?;

                if let Some(ref json) = json {
                    writeln!(out, "func (s *{}) UnmarshalJSON(buf []byte) error {{", name)?;
                    writeln!(out, "\tvar x struct{{ Tag string `json:{:?}` }}", json.0)?;
                    writeln!(out, "\tif err := json.Unmarshal(buf, &x); err != nil {{")?;
                    writeln!(out, "\t\treturn err")?;
                    writeln!(out, "\t}}")?;
                    writeln!(out, "\tswitch x.Tag {{")?;

                    for (name, val) in json.1.iter() {
                        writeln!(out, "\tcase {:?}:", name)?;
                        write!(out, "\t\tvar data ")?;
                        self.serialize_subschema(out, &val)?;
                        writeln!(out)?;
                        writeln!(
                            out,
                            "\t\tif err := json.Unmarshal(buf, &data); err != nil {{"
                        )?;
                        writeln!(out, "\t\t\treturn err")?;
                        writeln!(out, "\t\t}}")?;
                        writeln!(out, "\t\ts.Val = data")?;
                    }

                    writeln!(out, "\t}}")?;
                    writeln!(out, "\ts.{} = x.Tag", json.0.to_class_case())?;
                    writeln!(out, "\treturn nil")?;
                    writeln!(out, "}}")?;

                    writeln!(out, "func (s {}) MarshalJSON() ([]byte, error) {{", name)?;
                    writeln!(out, "\tswitch val := s.Val.(type) {{")?;

                    for (_, val) in json.1.iter() {
                        write!(out, "\tcase ")?;
                        self.serialize_subschema(out, &val)?;
                        writeln!(out, ":")?;
                        writeln!(out, "\t\tvar data struct{{")?;
                        writeln!(out, "\t\t\tTag string `json:{:?}`", json.0)?;
                        write!(out, "\t\t\t")?;
                        self.serialize_subschema(out, &val)?;
                        writeln!(out)?;
                        writeln!(out, "\t\t}}")?;
                        write!(out, "\t\tdata.")?;
                        self.serialize_subschema(out, &val)?;
                        writeln!(out, " = val")?;
                        writeln!(out, "\t\tdata.Tag = s.{}", json.0.to_class_case())?;
                        writeln!(out, "\t\treturn json.Marshal(data)")?;
                    }

                    writeln!(out, "\t}}")?;
                    writeln!(out, "\tpanic(\"invalid discriminator tag\")")?;
                    writeln!(out, "}}")?;
                }
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
    Time,

    // A string literal.
    Literal(String),

    // An identifier.
    Identifier(String),

    // An array with elements of some type.
    Array(Box<Ast>),

    // A map from strings to some type.
    Map(Box<Ast>),

    // A type declaration.
    Type(String, Box<Ast>),

    // A var declaration. First parameter is the name, second is the type, third
    // is the value.
    Var(String, Box<Ast>, Box<Ast>),

    // An interface with a name and properties.
    //
    // If the third argument is present, this indicates that MarshalJSON and
    // UnmarshalJSON should be overridden to act as a discriminated union. The
    // first parameter is the name of the tag, and the second parameter is a
    // mapping from tag values to corresponding type identifiers.
    Struct(
        String,
        Vec<Property>,
        Option<(String, HashMap<String, Ast>)>,
    ),
}

#[derive(Debug)]
pub struct Property {
    name: String,
    json_name: String,
    value: Ast,
}
