# init

Create a new Vexil schema file with a starter template.

## Usage

```sh
vexilc init [name]
```

## Example

```sh
$ vexilc init myapp
Created myapp.vexil
```

This creates `myapp.vexil` with a starter schema:

```vexil
namespace myapp

message Hello {
    name     @0 : string
    greeting @1 : string
    count    @2 : u32
}
```

## Notes

- The command refuses to overwrite an existing file
- The name becomes both the filename and the namespace
