//! Diagnostics-only Language Server Protocol adapter.

use std::collections::HashMap;
use std::error::Error;

use lsp_server::{Connection, Message, Response, ResponseError};
use lsp_types::{
    DiagnosticSeverity, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, InitializeParams, InitializeResult, NumberOrString, Position,
    PositionEncodingKind, PublishDiagnosticsParams, Range, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use vexil_lang::diagnostic::{Diagnostic as CompilerDiagnostic, Severity};

type LspResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

const DID_OPEN: &str = "textDocument/didOpen";
const DID_CHANGE: &str = "textDocument/didChange";
const DID_CLOSE: &str = "textDocument/didClose";
const PUBLISH_DIAGNOSTICS: &str = "textDocument/publishDiagnostics";

#[derive(Debug)]
struct Document {
    text: String,
    version: i32,
}

#[derive(Debug, PartialEq, Eq)]
enum ServerExit {
    Clean,
    WithoutShutdown,
}

pub(crate) fn cmd_lsp() -> i32 {
    match run_stdio() {
        Ok(ServerExit::Clean) => 0,
        Ok(ServerExit::WithoutShutdown) => 1,
        Err(error) => {
            eprintln!("vexilc lsp: {error}");
            1
        }
    }
}

fn run_stdio() -> LspResult<ServerExit> {
    let (connection, io_threads) = Connection::stdio();
    let exit = run_connection(connection)?;
    io_threads.join()?;
    Ok(exit)
}

fn run_connection(connection: Connection) -> LspResult<ServerExit> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let _: InitializeParams = serde_json::from_value(initialize_params)?;
    let initialize_result = InitializeResult {
        capabilities: server_capabilities(),
        server_info: Some(ServerInfo {
            name: "vexilc".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    };
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

    let mut documents = HashMap::new();
    loop {
        let message = match connection.receiver.recv() {
            Ok(message) => message,
            Err(_) => return Ok(ServerExit::WithoutShutdown),
        };
        match message {
            Message::Request(request) => {
                if connection.handle_shutdown(&request)? {
                    return Ok(ServerExit::Clean);
                }
                let response = Response {
                    id: request.id,
                    response_result: Err(ResponseError {
                        code: lsp_server::ErrorCode::MethodNotFound as i32,
                        message: format!("unsupported request: {}", request.method),
                        data: None,
                    }),
                };
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(notification) => {
                if notification.method == "exit" {
                    return Ok(ServerExit::WithoutShutdown);
                }
                let method = notification.method.clone();
                if let Err(error) = handle_notification(&connection, notification, &mut documents) {
                    eprintln!("vexilc lsp: invalid {method} notification: {error}");
                }
            }
            Message::Response(_) => {}
        }
    }
}

fn server_capabilities() -> ServerCapabilities {
    ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF16),
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: None,
            },
        )),
        ..ServerCapabilities::default()
    }
}

fn handle_notification(
    connection: &Connection,
    notification: lsp_server::Notification,
    documents: &mut HashMap<String, Document>,
) -> LspResult<()> {
    match notification.method.as_str() {
        DID_OPEN => {
            let params: DidOpenTextDocumentParams = serde_json::from_value(notification.params)?;
            let uri = params.text_document.uri;
            let document = Document {
                text: params.text_document.text,
                version: params.text_document.version,
            };
            publish_document_diagnostics(connection, uri.clone(), &document)?;
            documents.insert(uri.as_str().to_owned(), document);
        }
        DID_CHANGE => {
            let params: DidChangeTextDocumentParams = serde_json::from_value(notification.params)?;
            let [change] = params.content_changes.as_slice() else {
                return Err("full synchronization requires exactly one content change".into());
            };
            if change.range.is_some() || change.range_length.is_some() {
                return Err("incremental content changes are not supported".into());
            }
            let uri = params.text_document.uri;
            let document = Document {
                text: change.text.clone(),
                version: params.text_document.version,
            };
            publish_document_diagnostics(connection, uri.clone(), &document)?;
            documents.insert(uri.as_str().to_owned(), document);
        }
        DID_CLOSE => {
            let params: DidCloseTextDocumentParams = serde_json::from_value(notification.params)?;
            documents.remove(params.text_document.uri.as_str());
            publish_diagnostics(connection, params.text_document.uri, Vec::new(), None)?;
        }
        _ => {}
    }
    Ok(())
}

fn publish_document_diagnostics(
    connection: &Connection,
    uri: Uri,
    document: &Document,
) -> LspResult<()> {
    let result = vexil_lang::compile(&document.text);
    let diagnostics = result
        .diagnostics
        .iter()
        .map(|diagnostic| compiler_diagnostic_to_lsp(&document.text, diagnostic))
        .collect();
    publish_diagnostics(connection, uri, diagnostics, Some(document.version))
}

fn publish_diagnostics(
    connection: &Connection,
    uri: Uri,
    diagnostics: Vec<lsp_types::Diagnostic>,
    version: Option<i32>,
) -> LspResult<()> {
    let params = PublishDiagnosticsParams::new(uri, diagnostics, version);
    let notification = lsp_server::Notification {
        method: PUBLISH_DIAGNOSTICS.to_owned(),
        params: serde_json::to_value(params)?,
    };
    connection
        .sender
        .send(Message::Notification(notification))?;
    Ok(())
}

fn compiler_diagnostic_to_lsp(
    source: &str,
    diagnostic: &CompilerDiagnostic,
) -> lsp_types::Diagnostic {
    let severity = match diagnostic.severity {
        Severity::Error => DiagnosticSeverity::ERROR,
        Severity::Warning => DiagnosticSeverity::WARNING,
    };
    let mut message = diagnostic.message.clone();
    for note in &diagnostic.notes {
        message.push('\n');
        message.push_str(&note.format());
    }
    lsp_types::Diagnostic {
        range: byte_span_to_range(
            source,
            diagnostic.span.offset as usize,
            diagnostic.span.len as usize,
        ),
        severity: Some(severity),
        code: Some(NumberOrString::String(diagnostic.code.as_str().to_owned())),
        code_description: None,
        source: Some("vexilc".to_owned()),
        message,
        related_information: None,
        tags: None,
        data: None,
    }
}

fn byte_span_to_range(source: &str, offset: usize, length: usize) -> Range {
    let end = offset.saturating_add(length);
    Range::new(
        byte_offset_to_position(source, offset),
        byte_offset_to_position(source, end),
    )
}

fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let mut safe_offset = offset.min(source.len());
    while !source.is_char_boundary(safe_offset) {
        safe_offset = safe_offset.saturating_sub(1);
    }

    let mut line = 0_u32;
    let mut character = 0_u32;
    let mut byte_index = 0;
    let bytes = source.as_bytes();
    while byte_index < safe_offset {
        if bytes[byte_index] == b'\r' {
            let line_ending_len = if bytes.get(byte_index + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            if safe_offset < byte_index + line_ending_len {
                break;
            }
            line = line.saturating_add(1);
            character = 0;
            byte_index += line_ending_len;
        } else if bytes[byte_index] == b'\n' {
            line = line.saturating_add(1);
            character = 0;
            byte_index += 1;
        } else {
            let Some(ch) = source[byte_index..].chars().next() else {
                break;
            };
            character = character.saturating_add(ch.len_utf16() as u32);
            byte_index += ch.len_utf8();
        }
    }
    Position::new(line, character)
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use lsp_server::{Message, Notification, Request, RequestId};
    use lsp_types::{
        DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
        PublishDiagnosticsParams, TextDocumentContentChangeEvent, TextDocumentIdentifier,
        TextDocumentItem, Uri, VersionedTextDocumentIdentifier,
    };
    use vexil_lang::diagnostic::{ErrorClass, Note};
    use vexil_lang::span::Span;

    use super::*;

    fn uri() -> Uri {
        "file:///vexil/lsp-test.vexil"
            .parse()
            .expect("valid test URI")
    }

    fn send_notification(connection: &Connection, method: &str, params: impl serde::Serialize) {
        connection
            .sender
            .send(Message::Notification(Notification {
                method: method.to_owned(),
                params: serde_json::to_value(params).expect("serializable notification"),
            }))
            .expect("server connection remains open");
    }

    fn receive_message(connection: &Connection) -> Message {
        connection
            .receiver
            .recv_timeout(Duration::from_secs(2))
            .expect("server responds without hanging")
    }

    fn receive_diagnostics(connection: &Connection) -> PublishDiagnosticsParams {
        let Message::Notification(notification) = receive_message(connection) else {
            panic!("expected diagnostics notification");
        };
        assert_eq!(notification.method, PUBLISH_DIAGNOSTICS);
        serde_json::from_value(notification.params).expect("valid diagnostics payload")
    }

    #[test]
    fn byte_offsets_map_to_utf16_positions_and_clamp_safely() {
        let source = "a😀é\r\nβ";

        assert_eq!(byte_offset_to_position(source, 0), Position::new(0, 0));
        assert_eq!(byte_offset_to_position(source, 1), Position::new(0, 1));
        assert_eq!(byte_offset_to_position(source, 5), Position::new(0, 3));
        assert_eq!(byte_offset_to_position(source, 7), Position::new(0, 4));
        assert_eq!(byte_offset_to_position(source, 8), Position::new(0, 4));
        assert_eq!(byte_offset_to_position(source, 9), Position::new(1, 0));
        assert_eq!(
            byte_offset_to_position(source, source.len()),
            Position::new(1, 1)
        );
        assert_eq!(
            byte_offset_to_position(source, usize::MAX),
            Position::new(1, 1)
        );
        assert_eq!(byte_offset_to_position(source, 3), Position::new(0, 1));

        let lone_cr = "ab\rc";
        assert_eq!(byte_offset_to_position(lone_cr, 2), Position::new(0, 2));
        assert_eq!(byte_offset_to_position(lone_cr, 3), Position::new(1, 0));
        assert_eq!(byte_offset_to_position(lone_cr, 4), Position::new(1, 1));

        assert_eq!(
            byte_span_to_range(source, 1, 6),
            Range::new(Position::new(0, 1), Position::new(0, 4))
        );
        assert_eq!(
            byte_span_to_range(source, usize::MAX, usize::MAX),
            Range::new(Position::new(1, 1), Position::new(1, 1))
        );
    }

    #[test]
    fn compiler_diagnostics_preserve_code_severity_notes_and_range() {
        let source = "a😀 field";
        let compiler = CompilerDiagnostic::warning(
            Span::new(6, 5),
            ErrorClass::FieldNameInvalid,
            "invalid field",
        )
        .with_note(Note::Help("choose a lower-case name".to_owned()));

        let diagnostic = compiler_diagnostic_to_lsp(source, &compiler);

        assert_eq!(
            diagnostic.range,
            Range::new(Position::new(0, 4), Position::new(0, 9))
        );
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String(compiler.code.as_str().to_owned()))
        );
        assert_eq!(diagnostic.source.as_deref(), Some("vexilc"));
        assert_eq!(
            diagnostic.message,
            "invalid field\nhelp: choose a lower-case name"
        );
    }

    #[test]
    fn protocol_publishes_versions_clears_and_rejects_unsupported_requests() {
        let (server, client) = Connection::memory();
        let server_thread = thread::spawn(move || run_connection(server));

        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(1),
                method: "initialize".to_owned(),
                params: serde_json::json!({"capabilities": {}}),
            }))
            .expect("initialize request sent");
        let Message::Response(initialize) = receive_message(&client) else {
            panic!("expected initialize response");
        };
        let initialize = initialize.response_result.expect("successful initialize");
        assert_eq!(initialize["serverInfo"]["name"], "vexilc");
        assert_eq!(initialize["capabilities"]["positionEncoding"], "utf-16");
        assert_eq!(
            initialize["capabilities"]["textDocumentSync"]["openClose"],
            true
        );
        assert_eq!(initialize["capabilities"]["textDocumentSync"]["change"], 1);
        assert_eq!(
            initialize["capabilities"]
                .as_object()
                .expect("capability object")
                .len(),
            2
        );
        send_notification(&client, "initialized", serde_json::json!({}));

        let invalid = "namespace demo\n# 😀 keeps byte and UTF-16 offsets distinct\nmessage Item {\n  value @0 : Missing\n}\n";
        send_notification(
            &client,
            DID_OPEN,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem::new(
                    uri(),
                    "vexil".to_owned(),
                    1,
                    invalid.to_owned(),
                ),
            },
        );
        let published = receive_diagnostics(&client);
        assert_eq!(published.uri, uri());
        assert_eq!(published.version, Some(1));
        let unknown_type = published
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.code == Some(NumberOrString::String("E050".to_owned()))
                    && diagnostic.severity == Some(DiagnosticSeverity::ERROR)
                    && diagnostic.source.as_deref() == Some("vexilc")
            })
            .unwrap_or_else(|| panic!("published diagnostics: {:#?}", published.diagnostics));
        assert_eq!(
            unknown_type.range,
            Range::new(Position::new(3, 13), Position::new(3, 20))
        );

        let valid = "namespace demo\nmessage Item {\n  value @0 : u32\n}\n";
        send_notification(
            &client,
            DID_CHANGE,
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier::new(uri(), 2),
                content_changes: vec![TextDocumentContentChangeEvent {
                    range: None,
                    range_length: None,
                    text: valid.to_owned(),
                }],
            },
        );
        let cleared_by_change = receive_diagnostics(&client);
        assert_eq!(cleared_by_change.version, Some(2));
        assert!(cleared_by_change.diagnostics.is_empty());

        send_notification(
            &client,
            DID_CLOSE,
            DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier::new(uri()),
            },
        );
        let cleared_by_close = receive_diagnostics(&client);
        assert_eq!(cleared_by_close.version, None);
        assert!(cleared_by_close.diagnostics.is_empty());

        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(2),
                method: "textDocument/hover".to_owned(),
                params: serde_json::json!({}),
            }))
            .expect("unsupported request sent");
        let Message::Response(unsupported) = receive_message(&client) else {
            panic!("expected unsupported response");
        };
        let unsupported = unsupported
            .response_result
            .expect_err("method is unsupported");
        assert_eq!(
            unsupported.code,
            lsp_server::ErrorCode::MethodNotFound as i32
        );

        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(3),
                method: "shutdown".to_owned(),
                params: serde_json::Value::Null,
            }))
            .expect("shutdown request sent");
        let Message::Response(shutdown) = receive_message(&client) else {
            panic!("expected shutdown response");
        };
        assert!(shutdown.response_result.is_ok());
        send_notification(&client, "exit", serde_json::Value::Null);
        let exit = server_thread
            .join()
            .expect("server thread does not panic")
            .expect("server exits cleanly");
        assert_eq!(exit, ServerExit::Clean);
    }

    #[test]
    fn bare_exit_terminates_with_abnormal_status() {
        let (server, client) = Connection::memory();
        let server_thread = thread::spawn(move || run_connection(server));

        client
            .sender
            .send(Message::Request(Request {
                id: RequestId::from(1),
                method: "initialize".to_owned(),
                params: serde_json::json!({"capabilities": {}}),
            }))
            .expect("initialize request sent");
        let Message::Response(initialize) = receive_message(&client) else {
            panic!("expected initialize response");
        };
        assert!(initialize.response_result.is_ok());
        send_notification(&client, "initialized", serde_json::json!({}));
        send_notification(&client, "exit", serde_json::Value::Null);

        let exit = server_thread
            .join()
            .expect("server thread does not panic")
            .expect("server handles bare exit");
        assert_eq!(exit, ServerExit::WithoutShutdown);
    }
}
