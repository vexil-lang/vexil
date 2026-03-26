use crate::error::VxError;
use crate::lexer::{Lexer, Span, Spanned, Token};
use crate::Value;
use std::collections::BTreeMap;
use vexil_lang::ast::{PrimitiveType, SemanticType};
use vexil_lang::ir::{ConfigDef, EnumDef, FlagsDef, MessageDef, UnionDef};
use vexil_lang::{CompiledSchema, ResolvedType, TypeDef};

struct Parser<'s> {
    tokens: Vec<Spanned>,
    pos: usize,
    schema: &'s CompiledSchema,
    file: String,
    errors: Vec<VxError>,
}

impl<'s> Parser<'s> {
    fn new(tokens: Vec<Spanned>, schema: &'s CompiledSchema, file: String) -> Self {
        Self {
            tokens,
            pos: 0,
            schema,
            file,
            errors: Vec::new(),
        }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof)
    }

    fn peek_span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|s| s.span)
            .unwrap_or(Span { line: 0, col: 0 })
    }

    fn advance(&mut self) -> &Token {
        let tok = self
            .tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect_ident(&mut self) -> Result<String, VxError> {
        let span = self.peek_span();
        match self.advance().clone() {
            Token::Ident(s) => Ok(s),
            tok => Err(VxError::Parse {
                file: self.file.clone(),
                line: span.line,
                col: span.col,
                message: format!("expected identifier, got {tok:?}"),
            }),
        }
    }

    fn expect_token(&mut self, expected: &Token) -> Result<(), VxError> {
        let span = self.peek_span();
        let got = self.advance().clone();
        if &got == expected {
            Ok(())
        } else {
            Err(VxError::Parse {
                file: self.file.clone(),
                line: span.line,
                col: span.col,
                message: format!("expected {expected:?}, got {got:?}"),
            })
        }
    }

    fn error(&self, msg: impl Into<String>) -> VxError {
        let span = self.peek_span();
        VxError::Parse {
            file: self.file.clone(),
            line: span.line,
            col: span.col,
            message: msg.into(),
        }
    }
}

/// Parse a `.vx` text file into a list of top-level values, guided by a compiled schema.
pub fn parse(source: &str, schema: &CompiledSchema) -> Result<Vec<Value>, Vec<VxError>> {
    let mut lexer = Lexer::new(source, "<input>");
    let tokens = lexer.lex_all().map_err(|e| vec![e])?;
    let mut parser = Parser::new(tokens, schema, "<input>".to_string());
    parser.parse_file()
}

impl<'s> Parser<'s> {
    fn parse_file(&mut self) -> Result<Vec<Value>, Vec<VxError>> {
        // Parse optional @schema directive
        if let Token::Directive(d) = self.peek().clone() {
            if d == "schema" {
                self.advance();
                // Consume the namespace string (skip it)
                match self.peek().clone() {
                    Token::StringLit(_) | Token::Ident(_) => {
                        self.advance();
                    }
                    _ => {}
                }
            } else if d == "version" {
                // version comes before schema — not standard, but skip
            } else {
                self.errors
                    .push(self.error(format!("unknown directive: @{d}")));
            }
        }

        // Parse optional @version directive
        if let Token::Directive(d) = self.peek().clone() {
            if d == "version" {
                self.advance();
                match self.peek().clone() {
                    Token::StringLit(_) | Token::Ident(_) => {
                        self.advance();
                    }
                    _ => {}
                }
            }
        }

        // Parse top-level values: TypeName { ... }
        let mut values = Vec::new();
        while *self.peek() != Token::Eof {
            match self.peek().clone() {
                Token::Ident(type_name) => {
                    self.advance();
                    let type_id = match self.schema.registry.lookup(&type_name) {
                        Some(id) => id,
                        None => {
                            self.errors.push(VxError::UnknownType {
                                namespace: self
                                    .schema
                                    .namespace
                                    .iter()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join("."),
                                type_name: type_name.clone(),
                            });
                            // Try to skip to next type
                            while *self.peek() != Token::Eof
                                && !matches!(self.peek(), Token::Ident(_))
                            {
                                self.advance();
                            }
                            continue;
                        }
                    };
                    let type_def = match self.schema.registry.get(type_id) {
                        Some(td) => td,
                        None => {
                            self.errors
                                .push(self.error(format!("type not found: {type_name}")));
                            continue;
                        }
                    };
                    // Clone to avoid borrow conflict
                    let type_def = type_def.clone();
                    match self.parse_type_def_value(&type_def) {
                        Ok(v) => values.push(v),
                        Err(e) => self.errors.push(e),
                    }
                }
                tok => {
                    let err = self.error(format!("expected type name, got {tok:?}"));
                    self.errors.push(err);
                    self.advance();
                }
            }
        }

        if self.errors.is_empty() {
            Ok(values)
        } else {
            Err(std::mem::take(&mut self.errors))
        }
    }

    fn parse_type_def_value(&mut self, type_def: &TypeDef) -> Result<Value, VxError> {
        match type_def {
            TypeDef::Message(msg) => self.parse_message(msg),
            TypeDef::Enum(e) => self.parse_enum_value(e),
            TypeDef::Flags(f) => self.parse_flags_value(f),
            TypeDef::Union(u) => self.parse_union_value(u),
            TypeDef::Newtype(nt) => {
                let terminal = nt.terminal_type.clone();
                self.parse_resolved(&terminal)
            }
            TypeDef::Config(cfg) => self.parse_config(cfg),
            _ => Err(self.error("unknown type kind")),
        }
    }

    fn parse_resolved(&mut self, ty: &ResolvedType) -> Result<Value, VxError> {
        match ty {
            ResolvedType::Primitive(p) => self.parse_primitive(*p),
            ResolvedType::SubByte(_) => {
                // Parse as integer
                match self.peek().clone() {
                    Token::IntLit(n) => {
                        self.advance();
                        Ok(Value::U64(n as u64))
                    }
                    tok => Err(self.error(format!("expected integer for sub-byte, got {tok:?}"))),
                }
            }
            ResolvedType::Semantic(s) => self.parse_semantic(*s),
            ResolvedType::Named(type_id) => {
                let td = self
                    .schema
                    .registry
                    .get(*type_id)
                    .ok_or_else(|| self.error("unknown type id"))?
                    .clone();
                self.parse_type_def_value(&td)
            }
            ResolvedType::Optional(inner) => {
                let inner = inner.clone();
                if let Token::Ident(s) = self.peek().clone() {
                    if s == "none" {
                        self.advance();
                        return Ok(Value::None);
                    }
                    if s == "some" {
                        self.advance();
                        self.expect_token(&Token::LParen)?;
                        let v = self.parse_resolved(&inner)?;
                        self.expect_token(&Token::RParen)?;
                        return Ok(Value::Some(Box::new(v)));
                    }
                }
                // Implicit Some: bare value
                let v = self.parse_resolved(&inner)?;
                Ok(Value::Some(Box::new(v)))
            }
            ResolvedType::Array(elem) => {
                let elem = elem.clone();
                self.expect_token(&Token::LBracket)?;
                let mut items = Vec::new();
                while *self.peek() != Token::RBracket && *self.peek() != Token::Eof {
                    items.push(self.parse_resolved(&elem)?);
                    if *self.peek() == Token::Comma {
                        self.advance();
                    }
                }
                self.expect_token(&Token::RBracket)?;
                Ok(Value::Array(items))
            }
            ResolvedType::Map(key_ty, val_ty) => {
                let key_ty = key_ty.clone();
                let val_ty = val_ty.clone();
                self.expect_token(&Token::LBrace)?;
                let mut entries = Vec::new();
                while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
                    let k = self.parse_resolved(&key_ty)?;
                    self.expect_token(&Token::Colon)?;
                    let v = self.parse_resolved(&val_ty)?;
                    entries.push((k, v));
                    if *self.peek() == Token::Comma {
                        self.advance();
                    }
                }
                self.expect_token(&Token::RBrace)?;
                Ok(Value::Map(entries))
            }
            ResolvedType::Result(ok_ty, err_ty) => {
                let ok_ty = ok_ty.clone();
                let err_ty = err_ty.clone();
                match self.peek().clone() {
                    Token::Ident(s) if s == "ok" => {
                        self.advance();
                        self.expect_token(&Token::LParen)?;
                        let v = self.parse_resolved(&ok_ty)?;
                        self.expect_token(&Token::RParen)?;
                        Ok(Value::Ok(Box::new(v)))
                    }
                    Token::Ident(s) if s == "err" => {
                        self.advance();
                        self.expect_token(&Token::LParen)?;
                        let v = self.parse_resolved(&err_ty)?;
                        self.expect_token(&Token::RParen)?;
                        Ok(Value::Err(Box::new(v)))
                    }
                    tok => Err(self.error(format!("expected ok(...) or err(...), got {tok:?}"))),
                }
            }
            _ => Err(self.error(format!("unsupported type: {ty:?}"))),
        }
    }

    fn parse_primitive(&mut self, prim: PrimitiveType) -> Result<Value, VxError> {
        match (self.peek().clone(), prim) {
            (Token::Ident(s), PrimitiveType::Bool) if s == "true" => {
                self.advance();
                Ok(Value::Bool(true))
            }
            (Token::Ident(s), PrimitiveType::Bool) if s == "false" => {
                self.advance();
                Ok(Value::Bool(false))
            }
            (Token::IntLit(n), PrimitiveType::U8) => {
                self.advance();
                Ok(Value::U8(n as u8))
            }
            (Token::IntLit(n), PrimitiveType::U16) => {
                self.advance();
                Ok(Value::U16(n as u16))
            }
            (Token::IntLit(n), PrimitiveType::U32) => {
                self.advance();
                Ok(Value::U32(n as u32))
            }
            (Token::IntLit(n), PrimitiveType::U64) => {
                self.advance();
                Ok(Value::U64(n as u64))
            }
            (Token::IntLit(n), PrimitiveType::I8) => {
                self.advance();
                Ok(Value::I8(n as i8))
            }
            (Token::IntLit(n), PrimitiveType::I16) => {
                self.advance();
                Ok(Value::I16(n as i16))
            }
            (Token::IntLit(n), PrimitiveType::I32) => {
                self.advance();
                Ok(Value::I32(n as i32))
            }
            (Token::IntLit(n), PrimitiveType::I64) => {
                self.advance();
                Ok(Value::I64(n as i64))
            }
            (Token::FloatLit(f), PrimitiveType::F32) => {
                self.advance();
                Ok(Value::F32(f as f32))
            }
            (Token::FloatLit(f), PrimitiveType::F64) => {
                self.advance();
                Ok(Value::F64(f))
            }
            // Also allow integers for floats
            (Token::IntLit(n), PrimitiveType::F32) => {
                self.advance();
                Ok(Value::F32(n as f32))
            }
            (Token::IntLit(n), PrimitiveType::F64) => {
                self.advance();
                Ok(Value::F64(n as f64))
            }
            (tok, _) => Err(self.error(format!("expected {prim:?}, got {tok:?}"))),
        }
    }

    fn parse_semantic(&mut self, sem: SemanticType) -> Result<Value, VxError> {
        match (self.peek().clone(), sem) {
            (Token::StringLit(s), SemanticType::String) => {
                self.advance();
                Ok(Value::String(s))
            }
            (Token::HexBytes(b), SemanticType::Bytes) => {
                self.advance();
                Ok(Value::Bytes(b))
            }
            (Token::Base64Bytes(b), SemanticType::Bytes) => {
                self.advance();
                Ok(Value::Bytes(b))
            }
            (Token::HexBytes(b), SemanticType::Hash) if b.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&b);
                self.advance();
                Ok(Value::Hash(arr))
            }
            (Token::HexBytes(b), SemanticType::Uuid) if b.len() == 16 => {
                let mut arr = [0u8; 16];
                arr.copy_from_slice(&b);
                self.advance();
                Ok(Value::Uuid(arr))
            }
            (Token::HexBytes(b), SemanticType::Rgb) if b.len() == 3 => {
                self.advance();
                Ok(Value::Rgb([b[0], b[1], b[2]]))
            }
            (Token::IntLit(ts), SemanticType::Timestamp) => {
                self.advance();
                Ok(Value::Timestamp(ts as i64))
            }
            (tok, _) => Err(self.error(format!("expected {sem:?} value, got {tok:?}"))),
        }
    }

    fn parse_message(&mut self, msg: &MessageDef) -> Result<Value, VxError> {
        // Clone fields to avoid borrow conflicts
        let fields_schema: Vec<(String, ResolvedType)> = msg
            .fields
            .iter()
            .map(|f| (f.name.to_string(), f.resolved_type.clone()))
            .collect();
        let msg_name = msg.name.to_string();

        self.expect_token(&Token::LBrace)?;
        let mut fields = BTreeMap::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            let field_name = self.expect_ident()?;
            self.expect_token(&Token::Colon)?;

            let field_type = fields_schema
                .iter()
                .find(|(name, _)| name == &field_name)
                .map(|(_, ty)| ty.clone());

            match field_type {
                Some(resolved_type) => {
                    let value = self.parse_resolved(&resolved_type)?;
                    fields.insert(field_name, value);
                }
                None => {
                    self.errors.push(VxError::UnknownField {
                        type_name: msg_name.clone(),
                        field: field_name,
                    });
                    self.skip_value()?;
                }
            }
            if *self.peek() == Token::Comma {
                self.advance();
            }
        }
        self.expect_token(&Token::RBrace)?;
        Ok(Value::Message(fields))
    }

    fn parse_enum_value(&mut self, enum_def: &EnumDef) -> Result<Value, VxError> {
        let variant_names: Vec<String> = enum_def
            .variants
            .iter()
            .map(|v| v.name.to_string())
            .collect();
        let enum_name = enum_def.name.to_string();

        let name = self.expect_ident()?;
        if variant_names.iter().any(|v| v == &name) {
            Ok(Value::Enum(name))
        } else {
            Err(VxError::UnknownVariant {
                type_name: enum_name,
                variant: name,
            })
        }
    }

    fn parse_flags_value(&mut self, flags_def: &FlagsDef) -> Result<Value, VxError> {
        let bit_names: Vec<String> = flags_def.bits.iter().map(|b| b.name.to_string()).collect();
        let flags_name = flags_def.name.to_string();

        let mut names = Vec::new();
        let name = self.expect_ident()?;
        if !bit_names.iter().any(|b| b == &name) {
            return Err(VxError::UnknownVariant {
                type_name: flags_name,
                variant: name,
            });
        }
        names.push(name);
        while *self.peek() == Token::Pipe {
            self.advance();
            let name = self.expect_ident()?;
            if !bit_names.iter().any(|b| b == &name) {
                return Err(VxError::UnknownVariant {
                    type_name: flags_name,
                    variant: name,
                });
            }
            names.push(name);
        }
        Ok(Value::Flags(names))
    }

    fn parse_union_value(&mut self, union_def: &UnionDef) -> Result<Value, VxError> {
        // Pre-extract variant info to avoid borrow issues
        let union_name = union_def.name.to_string();
        let variants: Vec<(String, Vec<(String, ResolvedType)>)> = union_def
            .variants
            .iter()
            .map(|v| {
                let fields = v
                    .fields
                    .iter()
                    .map(|f| (f.name.to_string(), f.resolved_type.clone()))
                    .collect();
                (v.name.to_string(), fields)
            })
            .collect();

        let variant_name = self.expect_ident()?;
        let variant_fields = variants
            .iter()
            .find(|(name, _)| name == &variant_name)
            .map(|(_, fields)| fields.clone())
            .ok_or_else(|| VxError::UnknownVariant {
                type_name: union_name.clone(),
                variant: variant_name.clone(),
            })?;

        self.expect_token(&Token::LBrace)?;
        let mut fields = BTreeMap::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            let field_name = self.expect_ident()?;
            self.expect_token(&Token::Colon)?;
            let field_type = variant_fields
                .iter()
                .find(|(name, _)| name == &field_name)
                .map(|(_, ty)| ty.clone());

            match field_type {
                Some(resolved_type) => {
                    let value = self.parse_resolved(&resolved_type)?;
                    fields.insert(field_name, value);
                }
                None => {
                    self.errors.push(VxError::UnknownField {
                        type_name: format!("{union_name}::{variant_name}"),
                        field: field_name,
                    });
                    self.skip_value()?;
                }
            }
            if *self.peek() == Token::Comma {
                self.advance();
            }
        }
        self.expect_token(&Token::RBrace)?;
        Ok(Value::Union {
            variant: variant_name,
            fields,
        })
    }

    fn parse_config(&mut self, config: &ConfigDef) -> Result<Value, VxError> {
        let fields_schema: Vec<(String, ResolvedType)> = config
            .fields
            .iter()
            .map(|f| (f.name.to_string(), f.resolved_type.clone()))
            .collect();
        let config_name = config.name.to_string();

        self.expect_token(&Token::LBrace)?;
        let mut fields = BTreeMap::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            let field_name = self.expect_ident()?;
            self.expect_token(&Token::Colon)?;

            let field_type = fields_schema
                .iter()
                .find(|(name, _)| name == &field_name)
                .map(|(_, ty)| ty.clone());

            match field_type {
                Some(resolved_type) => {
                    let value = self.parse_resolved(&resolved_type)?;
                    fields.insert(field_name, value);
                }
                None => {
                    self.errors.push(VxError::UnknownField {
                        type_name: config_name.clone(),
                        field: field_name,
                    });
                    self.skip_value()?;
                }
            }
            if *self.peek() == Token::Comma {
                self.advance();
            }
        }
        self.expect_token(&Token::RBrace)?;
        Ok(Value::Message(fields))
    }

    /// Skip a value (used for unknown field recovery): consume until a reasonable boundary.
    fn skip_value(&mut self) -> Result<(), VxError> {
        match self.peek().clone() {
            Token::LBrace => {
                self.advance();
                let mut depth = 1usize;
                while depth > 0 && *self.peek() != Token::Eof {
                    match self.advance() {
                        Token::LBrace => depth += 1,
                        Token::RBrace => depth -= 1,
                        _ => {}
                    }
                }
            }
            Token::LBracket => {
                self.advance();
                let mut depth = 1usize;
                while depth > 0 && *self.peek() != Token::Eof {
                    match self.advance() {
                        Token::LBracket => depth += 1,
                        Token::RBracket => depth -= 1,
                        _ => {}
                    }
                }
            }
            _ => {
                self.advance();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexil_lang::diagnostic::Severity;

    fn compile_schema(source: &str) -> vexil_lang::CompiledSchema {
        let result = vexil_lang::compile(source);
        let has_errors = result
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error);
        assert!(!has_errors, "schema errors: {:?}", result.diagnostics);
        result.compiled.expect("schema should compile")
    }

    #[test]
    fn parse_simple_message() {
        let schema = compile_schema(
            r#"
            namespace test.parse
            message Point { x @0 : u32  y @1 : u32 }
        "#,
        );
        let input = r#"@schema "test.parse"
Point { x: 10 y: 20 }"#;
        let values = parse(input, &schema).unwrap();
        assert_eq!(values.len(), 1);
        if let Value::Message(fields) = &values[0] {
            assert_eq!(fields["x"], Value::U32(10));
            assert_eq!(fields["y"], Value::U32(20));
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn parse_optional_some_and_none() {
        let schema = compile_schema(
            r#"
            namespace test.parse.opt
            message Named { name @0 : optional<string> }
        "#,
        );

        let input_some = "@schema \"test.parse.opt\"\nNamed { name: \"hello\" }";
        let values = parse(input_some, &schema).unwrap();
        if let Value::Message(fields) = &values[0] {
            assert_eq!(
                fields["name"],
                Value::Some(Box::new(Value::String("hello".to_string())))
            );
        } else {
            panic!("expected Message");
        }

        let input_none = "@schema \"test.parse.opt\"\nNamed { name: none }";
        let values = parse(input_none, &schema).unwrap();
        if let Value::Message(fields) = &values[0] {
            assert_eq!(fields["name"], Value::None);
        } else {
            panic!("expected Message");
        }
    }

    #[test]
    fn parse_enum() {
        let schema = compile_schema(
            r#"
            namespace test.parse.enum
            enum Color { Red @0  Green @1  Blue @2 }
            message Pixel { color @0 : Color }
        "#,
        );
        let input = "@schema \"test.parse.enum\"\nPixel { color: Green }";
        let values = parse(input, &schema).unwrap();
        if let Value::Message(fields) = &values[0] {
            assert_eq!(fields["color"], Value::Enum("Green".to_string()));
        } else {
            panic!();
        }
    }

    #[test]
    fn parse_array() {
        let schema = compile_schema(
            r#"
            namespace test.parse.arr
            message Numbers { values @0 : array<u32> }
        "#,
        );
        let input = "@schema \"test.parse.arr\"\nNumbers { values: [1, 2, 3] }";
        let values = parse(input, &schema).unwrap();
        if let Value::Message(fields) = &values[0] {
            assert_eq!(
                fields["values"],
                Value::Array(vec![Value::U32(1), Value::U32(2), Value::U32(3)])
            );
        } else {
            panic!();
        }
    }

    #[test]
    fn parse_union() {
        let schema = compile_schema(
            r#"
            namespace test.parse.union
            union Shape {
                Circle @0 { radius @0 : f64 }
                Square @1 { side @0 : f64 }
            }
            message Canvas { shape @0 : Shape }
        "#,
        );
        let input = "@schema \"test.parse.union\"\nCanvas { shape: Circle { radius: 3.14 } }";
        let values = parse(input, &schema).unwrap();
        if let Value::Message(fields) = &values[0] {
            assert!(
                matches!(&fields["shape"], Value::Union { variant, .. } if variant == "Circle")
            );
        } else {
            panic!();
        }
    }

    #[test]
    fn parse_missing_schema_directive() {
        let schema = compile_schema(
            r#"
            namespace test.parse.noschema
            message Foo { x @0 : u32 }
        "#,
        );
        // No @schema directive — parser should still work (it's lenient)
        let input = "Foo { x: 1 }";
        // Should succeed (we don't require @schema for parsing)
        let result = parse(input, &schema);
        assert!(result.is_ok());
    }
}
