use duckdb::Connection;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::path::PathBuf;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    protocol_version: u32,
    request_id: String,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    protocol_version: u32,
    request_id: String,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
}

fn success(id: &str, result: serde_json::Value) -> Response {
    Response {
        protocol_version: 1,
        request_id: id.to_owned(),
        ok: true,
        result: Some(result),
        error: None,
    }
}

fn failure(id: &str, code: &str, message: &str) -> Response {
    Response {
        protocol_version: 1,
        request_id: id.to_owned(),
        ok: false,
        result: None,
        error: Some(ErrorBody {
            code: code.to_owned(),
            message: message.to_owned(),
        }),
    }
}

fn write_response(stdout: &mut impl Write, response: &Response) {
    writeln!(stdout, "{}", serde_json::to_string(response).unwrap()).unwrap();
    stdout.flush().unwrap();
}

fn main() {
    eprintln!("VORA_E021_SIDECAR_START pid={}", std::process::id());
    let mut memory: Option<Connection> = None;
    let mut persistent: Option<Connection> = None;
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout().lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("stdin error: {error}");
                break;
            }
        };
        let request: Request = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                write_response(
                    &mut stdout,
                    &failure("unknown", "INVALID_REQUEST", &error.to_string()),
                );
                continue;
            }
        };
        if request.protocol_version != 1 {
            write_response(
                &mut stdout,
                &failure(
                    &request.request_id,
                    "UNSUPPORTED_PROTOCOL",
                    "protocolVersion must be 1",
                ),
            );
            continue;
        }

        let response = match request.method.as_str() {
            "health" => success(
                &request.request_id,
                serde_json::json!({
                    "status": "ready",
                    "pid": std::process::id(),
                    "engine": "duckdb"
                }),
            ),
            "create_synthetic_database" => {
                let database = request
                    .params
                    .get("database")
                    .and_then(|value| value.as_str())
                    .unwrap_or(":memory:");
                if database == ":memory:" {
                    match Connection::open_in_memory() {
                        Ok(connection) => {
                            memory = Some(connection);
                            success(
                                &request.request_id,
                                serde_json::json!({"database": ":memory:"}),
                            )
                        }
                        Err(error) => failure(
                            &request.request_id,
                            "DATABASE_ERROR",
                            &error.to_string(),
                        ),
                    }
                } else {
                    match Connection::open(PathBuf::from(database)) {
                        Ok(connection) => {
                            persistent = Some(connection);
                            success(
                                &request.request_id,
                                serde_json::json!({"database": database}),
                            )
                        }
                        Err(error) => failure(
                            &request.request_id,
                            "DATABASE_ERROR",
                            &error.to_string(),
                        ),
                    }
                }
            }
            "execute_approved_query" => {
                let database = request
                    .params
                    .get("database")
                    .and_then(|value| value.as_str())
                    .unwrap_or(":memory:");
                let sql = request
                    .params
                    .get("sql")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                let connection = if database == ":memory:" {
                    memory.as_ref()
                } else {
                    persistent.as_ref()
                };
                match (connection, sql) {
                    (Some(_), "") => failure(
                        &request.request_id,
                        "INVALID_PARAMS",
                        "sql is required",
                    ),
                    (None, _) => failure(
                        &request.request_id,
                        "DATABASE_NOT_OPEN",
                        "open a synthetic database first",
                    ),
                    (Some(connection), sql) => {
                        let normalized = sql.trim_start().to_ascii_uppercase();
                        let is_read_query = normalized.starts_with("SELECT")
                            || normalized.starts_with("WITH")
                            || normalized.starts_with("SHOW")
                            || normalized.starts_with("PRAGMA")
                            || normalized.starts_with("DESCRIBE")
                            || normalized.starts_with("SUMMARIZE")
                            || normalized.starts_with("EXPLAIN");
                        if !is_read_query {
                            match connection.execute_batch(sql) {
                                Ok(_) => success(
                                    &request.request_id,
                                    serde_json::json!({"executed": true}),
                                ),
                                Err(error) => failure(
                                    &request.request_id,
                                    "QUERY_ERROR",
                                    &error.to_string(),
                                ),
                            }
                        } else {
                            let mut statement = match connection.prepare(sql) {
                                Ok(statement) => statement,
                                Err(error) => {
                                    write_response(
                                        &mut stdout,
                                        &failure(
                                            &request.request_id,
                                            "QUERY_ERROR",
                                            &error.to_string(),
                                        ),
                                    );
                                    continue;
                                }
                            };
                            let column_count = statement.column_count();
                            let mut rows = match statement.query([]) {
                                Ok(rows) => rows,
                                Err(error) => {
                                    write_response(
                                        &mut stdout,
                                        &failure(
                                            &request.request_id,
                                            "QUERY_ERROR",
                                            &error.to_string(),
                                        ),
                                    );
                                    continue;
                                }
                            };
                            let mut values = Vec::new();
                            while let Ok(Some(row)) = rows.next() {
                                let mut object = serde_json::Map::new();
                                for index in 0..column_count {
                                    let name = format!("column{index}");
                                    if let Ok(value) = row.get::<_, f64>(index) {
                                        object.insert(name, serde_json::json!(value));
                                    } else if let Ok(value) = row.get::<_, i64>(index) {
                                        object.insert(name, serde_json::json!(value));
                                    } else if let Ok(value) = row.get::<_, String>(index) {
                                        object.insert(name, serde_json::json!(value));
                                    } else {
                                        object.insert(name, serde_json::Value::Null);
                                    }
                                }
                                values.push(serde_json::Value::Object(object));
                            }
                            success(
                                &request.request_id,
                                serde_json::json!({"rows": values}),
                            )
                        }
                    }
                }
            }
            "shutdown" => {
                write_response(
                    &mut stdout,
                    &success(
                        &request.request_id,
                        serde_json::json!({"status": "stopping"}),
                    ),
                );
                memory.take();
                persistent.take();
                break;
            }
            _ => failure(
                &request.request_id,
                "METHOD_NOT_ALLOWED",
                "method is not allowed",
            ),
        };
        write_response(&mut stdout, &response);
    }
    eprintln!("VORA_E021_SIDECAR_STOP");
}
