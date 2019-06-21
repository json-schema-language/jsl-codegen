# jsl-codegen

`jsl-codegen` generates data structures from JSON Schema Language. Define your
schema in a portable way with JSL, and then generate idiomatic code -- structs,
classes, interfaces, et cetera -- for any language.

## Supported Languages

`jsl-codegen` can output code for the following languages:

* TypeScript
* Golang
* Java

More targets can be added relatively easily. Just open a GitHub issue on this
project to make a feature request!

## Usage

As a quick example, here's how you create some TypeScript from a JSL schema:

```bash
jsl-codegen --ts-out=gen/typescript -- user.json
```

That will output some TypeScript into `gen/typescript/user.ts`.

## Example

If you're using TypeScript, `jsl-codegen` can convert a JSL schema like this:

```json
{
  "properties": {
    "name": { "type": "string" },
    "isAdmin": { "type": "boolean" }
  },
  "optionalProperties": {
    "favoriteNumbers": { "elements": { "type": "number" }}
  }
}
```

Into this:

```typescript
interface User {
  name: string;
  isAdmin: boolean;
  favoriteNumbers?: number[];
}
```

But using the same exact schema, you can also generate some Java:

```java
public class User {
  public String name;
  public boolean isAdmin;
  public List<Double> favoriteNumbers;
}
```

Or some Golang:

```go
type User struct {
  Name           string    `json:"name"`
  IsAdmin        bool      `json:"isAdmin"`
  FavoriteNumber []float64 `json:"favoriteNumbers"`
}
```

## Full Usage

JSL supports multiple output languages, and can output multiple languages at
once. As a consequence, all of the following parameters can be provided
simultaneously.

```text
JSON Schema Language Codegen 1.0
Generates code from a JSON Schema Language schema.

USAGE:
    jsl-codegen [OPTIONS] [--] <INPUT>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --java-out <java-out>    Java output directory.
        --java-pkg <java-pkg>    Java output package.
        --ts-file <ts-file>      Force a TypeScript file name, rather than inferring.
        --ts-out <ts-out>        TypeScript output directory.

ARGS:
    <INPUT>    Input JSON Schema Language schema
```
