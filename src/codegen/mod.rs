pub mod go;
pub mod java;
pub mod typescript;

use failure::Error;
use jsl::Schema;

pub trait Codegen
where
    Self: Sized,
{
    type Ast;

    fn args<'a, 'b>(app: clap::App<'a, 'b>) -> clap::App<'a, 'b>;
    fn from_args(matches: &clap::ArgMatches) -> Result<Option<Self>, Error>;
    fn transform(&self, schema: &Schema) -> Result<Self::Ast, Error>;
    fn serialize(&self, ast: &Self::Ast) -> Result<(), Error>;
}
