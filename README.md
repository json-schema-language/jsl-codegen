# jsl-codegen

`jsl-codegen` generates data structures from JSON Schema Language. Define your
schema in a portable way with JSL, and then generate idiomatic code -- structs,
classes, interfaces, et cetera -- for any language.

## Supported Languages

`jsl-codegen` can output code for the following languages:

* TypeScript
* Java

Each of those links takes you to documentation specific for each language. Since
each language

## Usage

Run `jsl-codegen --help` for details, but as a quick example, here's how you
create some TypeScript from a JSL schema:

```bash
jsl-codegen --ts-out=gen/typescript -- user.json
```

That will output some TypeScript into `gen/typescript`.

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

But using the same exact schema, you can also generate some Golang:

```golang
type User struct {
  Name             string  `json:"name"`
  IsAdmin          bool    `json:"isAdmin"`
  FavoriteNumbers []string `json:"favoriteNumbers"`
}
```

Or some Java:

```java
public class User {
  private String name;
  private boolean isAdmin;
  private List<double> favoriteNumbers;

  public User(String name, boolean isAdmin, List<double> favoriteNumbers) {
    this.name = name;
    this.isAdmin = isAdmin;
    this.favoriteNumbers = favoriteNumbers;
  }

  // Getters and setters omitted for brevity, but they're generated too.
}
```
