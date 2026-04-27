use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use chrono::Utc;

type HmacSha256 = Hmac<sha2::Sha256>;

fn hmac_sign(key: &[u8], msg: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Snowflake — session-token auth + query REST API
//
// The SQL API v2 (/api/v2/statements) only accepts OAuth Bearer or key-pair
// JWT auth — it rejects the session-token "Snowflake Token=..." format with
// error 390146.  The older query REST API (/queries/v1/query-request) fully
// supports session tokens and is exactly what the official Python connector
// uses under the hood.
//
// Flow:
//   1. POST /session/v1/login-request   →  get a short-lived session token
//   2. POST /queries/v1/query-request   →  execute INSERT with that token
// ---------------------------------------------------------------------------

/// Authenticate with Snowflake username + password and return a session token.
async fn snowflake_login(
    client: &Client,
    account: &str,
    username: &str,
    password: &str,
) -> Result<String, String> {
    let url = format!(
        "https://{account}.snowflakecomputing.com/session/v1/login-request\
         ?warehouse=&databaseName=&schemaName=&roleName="
    );

    let body = json!({
        "data": {
            "CLIENT_APP_ID":      "GenieRecorder",
            "CLIENT_APP_VERSION": "0.1.0",
            "SVN_REVISION":       "1",
            "ACCOUNT_NAME":       account.to_uppercase(),
            "LOGIN_NAME":         username,
            "PASSWORD":           password,
            "CLIENT_ENVIRONMENT": {
                "APPLICATION": "GenieRecorder",
                "OS":          "Windows_NT",
                "OS_VERSION":  "10.0"
            }
        }
    });

    let resp = client
        .post(&url)
        .header("user-agent", "GenieRecorder/0.1.0")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Snowflake login request failed: {e}"))?;

    let status = resp.status();
    let text   = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Snowflake login {status}: {text}"));
    }

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Snowflake login parse error: {e}\nBody: {text}"))?;

    if json["success"].as_bool() != Some(true) {
        let msg = json["message"].as_str().unwrap_or("unknown login error");
        return Err(format!("Snowflake login failed: {msg}"));
    }

    json["data"]["token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No session token in Snowflake login response: {text}"))
}

async fn insert_snowflake(metadata: &Value, cfg: &Value) -> Result<(), String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new());

    let account   = cfg["account"].as_str().unwrap_or("");
    let username  = cfg["username"].as_str().unwrap_or("");
    let password  = cfg["password"].as_str().unwrap_or("");
    let database  = cfg["database"].as_str().unwrap_or("");
    let schema    = cfg["schema"].as_str().unwrap_or("");
    let warehouse = cfg["warehouse"].as_str().unwrap_or("");
    let table     = cfg["table"].as_str().unwrap_or("genie_recordings");

    if account.is_empty() || username.is_empty() {
        return Err("Snowflake account or username not configured".into());
    }

    // Step 1 — authenticate and get a session token
    let token = snowflake_login(&client, account, username, password).await?;

    // Step 2 — build INSERT SQL with properly escaped literal values.
    // The query REST API does not support ? bindings, so we inline the values.
    fn esc(v: &Value) -> String {
        let s = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
        // Escape single quotes by doubling them (standard SQL)
        s.replace('\'', "''")
    }
    fn esc_num(v: &Value) -> String {
        match v {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }

    let sql = format!(
        "INSERT INTO \"{database}\".\"{schema}\".\"{table}\" \
         (opp_id, submission_date, duration_seconds, salesperson_name, salesperson_id, \
          mic_url, sys_url, mic_local_path, sys_local_path, sample_rate, channels) \
         VALUES ('{}','{}',{},'{}','{}','{}','{}','{}','{}',{},{})",
        esc(&metadata["opp_id"]),
        esc(&metadata["submission_date"]),
        esc_num(&metadata["duration_seconds"]),
        esc(&metadata["salesperson_name"]),
        esc(&metadata["salesperson_id"]),
        esc(&metadata["mic_url"]),
        esc(&metadata["sys_url"]),
        esc(&metadata["mic_local_path"]),
        esc(&metadata["sys_local_path"]),
        esc_num(&metadata["sample_rate"]),
        esc_num(&metadata["channels"]),
    );

    // Step 3 — POST to the query REST API (supports session-token auth)
    let request_id = Uuid::new_v4();
    let url = format!(
        "https://{account}.snowflakecomputing.com/queries/v1/query-request\
         ?requestId={request_id}&warehouse={warehouse}&databaseName={database}&schemaName={schema}"
    );

    let body = json!({
        "sqlText":             sql,
        "asyncExec":           false,
        "sequenceId":          1,
        "querySubmissionTime": Utc::now().timestamp_millis(),
        "parameters": {}
    });

    let resp = client
        .post(&url)
        .header("authorization", format!("Snowflake Token=\"{token}\""))
        .header("user-agent",    "GenieRecorder/0.1.0")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Snowflake query request failed: {e}"))?;

    let status = resp.status();
    let text   = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        return Err(format!("Snowflake query {status}: {text}"));
    }

    // The query API returns 200 even for SQL errors — check the payload
    let json: Value = serde_json::from_str(&text).unwrap_or_default();
    if json["success"].as_bool() == Some(false) {
        let msg = json["message"].as_str().unwrap_or(&text);
        return Err(format!("Snowflake SQL error: {msg}"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// BigQuery — tabledata.insertAll REST API
// ---------------------------------------------------------------------------
async fn get_gcp_token(service_account_json: &str) -> Result<String, String> {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    let sa: Value = serde_json::from_str(service_account_json)
        .map_err(|e| format!("Parse service account JSON: {e}"))?;

    let client_email = sa["client_email"].as_str().unwrap_or("");
    let private_key_pem = sa["private_key"].as_str().unwrap_or("");

    let now = chrono::Utc::now().timestamp();
    let claims = json!({
        "iss": client_email,
        "scope": "https://www.googleapis.com/auth/bigquery.insertdata",
        "aud": "https://oauth2.googleapis.com/token",
        "exp": now + 3600,
        "iat": now,
    });

    let key = EncodingKey::from_rsa_pem(private_key_pem.as_bytes())
        .map_err(|e| format!("RSA key error: {e}"))?;
    let header = Header::new(Algorithm::RS256);
    let jwt = encode(&header, &claims, &key).map_err(|e| format!("JWT encode error: {e}"))?;

    let client = Client::new();
    let resp: Value = client
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    resp["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("No access_token in GCP response: {resp}"))
}

async fn insert_bigquery(metadata: &Value, cfg: &Value) -> Result<(), String> {
    let project_id = cfg["projectId"].as_str().unwrap_or("");
    let dataset_id = cfg["datasetId"].as_str().unwrap_or("");
    let table_id = cfg["tableId"].as_str().unwrap_or("genie_recordings");
    let sa_key = cfg["serviceAccountKey"].as_str().unwrap_or("");

    let token = get_gcp_token(sa_key).await?;

    let url = format!(
        "https://bigquery.googleapis.com/bigquery/v2/projects/{project_id}/datasets/{dataset_id}/tables/{table_id}/insertAll"
    );

    let row = json!({
        "insertId": Uuid::new_v4().to_string(),
        "json": metadata
    });

    let body = json!({ "rows": [row] });

    let client = Client::new();
    client
        .post(&url)
        .header("authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// ClickHouse — HTTP interface, JSONEachRow
// ---------------------------------------------------------------------------
async fn insert_clickhouse(metadata: &Value, cfg: &Value) -> Result<(), String> {
    let client = Client::new();
    let host = cfg["host"].as_str().unwrap_or("localhost");
    let port = cfg["port"].as_u64().unwrap_or(8123);
    let database = cfg["database"].as_str().unwrap_or("default");
    let table = cfg["table"].as_str().unwrap_or("genie_recordings");
    let username = cfg["username"].as_str().unwrap_or("default");
    let password = cfg["password"].as_str().unwrap_or("");

    let url = format!(
        "http://{host}:{port}/?query=INSERT+INTO+{database}.{table}+FORMAT+JSONEachRow"
    );

    let row = metadata.to_string();

    client
        .post(&url)
        .header("X-ClickHouse-User", username)
        .header("X-ClickHouse-Key", password)
        .body(row)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Databricks — SQL Statement Execution API
// ---------------------------------------------------------------------------
async fn insert_databricks(metadata: &Value, cfg: &Value) -> Result<(), String> {
    let client = Client::new();
    let host = cfg["host"].as_str().unwrap_or("");
    let http_path = cfg["httpPath"].as_str().unwrap_or("");
    let token = cfg["accessToken"].as_str().unwrap_or("");
    let catalog = cfg["catalog"].as_str().unwrap_or("main");
    let schema = cfg["schema"].as_str().unwrap_or("default");
    let table = cfg["table"].as_str().unwrap_or("genie_recordings");

    // Extract warehouse id from http_path: /sql/1.0/warehouses/{id}
    let warehouse_id = http_path
        .split('/')
        .last()
        .unwrap_or("")
        .to_string();

    let url = format!("https://{host}/api/2.0/sql/statements");

    let stmt = format!(
        "INSERT INTO {catalog}.{schema}.{table} \
         (opp_id, submission_date, duration_seconds, salesperson_name, salesperson_id, \
          mic_url, sys_url, mic_local_path, sys_local_path, sample_rate, channels) \
         VALUES (:opp_id, :submission_date, :duration_seconds, :salesperson_name, :salesperson_id, \
                 :mic_url, :sys_url, :mic_local_path, :sys_local_path, :sample_rate, :channels)"
    );

    let body = json!({
        "warehouse_id": warehouse_id,
        "statement": stmt,
        "parameters": [
            { "name": "opp_id",            "value": metadata["opp_id"],            "type": "STRING" },
            { "name": "submission_date",   "value": metadata["submission_date"],   "type": "STRING" },
            { "name": "duration_seconds",  "value": metadata["duration_seconds"].to_string(),  "type": "INT" },
            { "name": "salesperson_name",  "value": metadata["salesperson_name"],  "type": "STRING" },
            { "name": "salesperson_id",    "value": metadata["salesperson_id"],    "type": "STRING" },
            { "name": "mic_url",           "value": metadata["mic_url"],           "type": "STRING" },
            { "name": "sys_url",           "value": metadata["sys_url"],           "type": "STRING" },
            { "name": "mic_local_path",    "value": metadata["mic_local_path"],    "type": "STRING" },
            { "name": "sys_local_path",    "value": metadata["sys_local_path"],    "type": "STRING" },
            { "name": "sample_rate",       "value": metadata["sample_rate"].to_string(),       "type": "INT" },
            { "name": "channels",          "value": metadata["channels"].to_string(),          "type": "INT" }
        ]
    });

    client
        .post(&url)
        .header("authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Amazon Redshift Data API — AWS SigV4 signed
// ---------------------------------------------------------------------------
async fn insert_redshift(metadata: &Value, cfg: &Value) -> Result<(), String> {
    let client = Client::new();
    let cluster_id = cfg["host"].as_str().unwrap_or("");
    let region = cfg["region"].as_str().unwrap_or("us-east-1");
    let database = cfg["database"].as_str().unwrap_or("");
    let schema = cfg["schema"].as_str().unwrap_or("public");
    let table = cfg["table"].as_str().unwrap_or("genie_recordings");
    let db_user = cfg["username"].as_str().unwrap_or("");
    let access_key = cfg["accessKeyId"].as_str().unwrap_or("");
    let secret_key = cfg["secretAccessKey"].as_str().unwrap_or("");

    let sql = format!(
        "INSERT INTO {schema}.{table} \
         (opp_id, submission_date, duration_seconds, salesperson_name, salesperson_id, \
          mic_url, sys_url, mic_local_path, sys_local_path, sample_rate, channels) \
         VALUES ('{}', '{}', {}, '{}', '{}', '{}', '{}', '{}', '{}', {}, {})",
        metadata["opp_id"].as_str().unwrap_or(""),
        metadata["submission_date"].as_str().unwrap_or(""),
        metadata["duration_seconds"].as_i64().unwrap_or(0),
        metadata["salesperson_name"].as_str().unwrap_or(""),
        metadata["salesperson_id"].as_str().unwrap_or(""),
        metadata["mic_url"].as_str().unwrap_or(""),
        metadata["sys_url"].as_str().unwrap_or(""),
        metadata["mic_local_path"].as_str().unwrap_or(""),
        metadata["sys_local_path"].as_str().unwrap_or(""),
        metadata["sample_rate"].as_i64().unwrap_or(44100),
        metadata["channels"].as_i64().unwrap_or(1),
    );

    let payload = json!({
        "ClusterIdentifier": cluster_id,
        "Database": database,
        "DbUser": db_user,
        "Sql": sql
    });

    let body_str = payload.to_string();
    let body_bytes = body_str.as_bytes();
    let payload_hash = hex::encode(Sha256::digest(body_bytes));

    let host = format!("redshift-data.{region}.amazonaws.com");
    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    let canonical_headers = format!(
        "content-type:application/x-amz-json-1.1\nhost:{host}\nx-amz-date:{amz_date}\nx-amz-target:RedshiftData.ExecuteStatement\n"
    );
    let signed_headers = "content-type;host;x-amz-date;x-amz-target";

    let canonical_request = format!(
        "POST\n/\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let cr_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let scope = format!("{date_stamp}/{region}/redshift-data/aws4_request");
    let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{cr_hash}");

    let k_secret = format!("AWS4{secret_key}");
    let k_date = hmac_sign(k_secret.as_bytes(), &date_stamp);
    let k_region = hmac_sign(&k_date, region);
    let k_service = hmac_sign(&k_region, "redshift-data");
    let k_signing = hmac_sign(&k_service, "aws4_request");
    let signature = hex::encode({
        let mut mac = HmacSha256::new_from_slice(&k_signing).expect("HMAC key error");
        mac.update(string_to_sign.as_bytes());
        mac.finalize().into_bytes()
    });

    let auth = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    let url = format!("https://{host}/");
    client
        .post(&url)
        .header("host", &host)
        .header("x-amz-date", &amz_date)
        .header("x-amz-target", "RedshiftData.ExecuteStatement")
        .header("content-type", "application/x-amz-json-1.1")
        .header("authorization", &auth)
        .body(body_str)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Public dispatcher
// ---------------------------------------------------------------------------
pub async fn insert(metadata: &Value, warehouse_config: &Value) -> Result<(), String> {
    // warehouse_config is config.warehouse, which has shape:
    //   { "provider": "snowflake", "snowflake": { "account": ..., ... }, "bigquery": { ... }, ... }
    // Each insert_* fn receives only the nested provider-specific object.
    let provider = warehouse_config["provider"].as_str().unwrap_or("none");
    match provider {
        "snowflake"  => insert_snowflake(metadata,  &warehouse_config["snowflake"]).await,
        "bigquery"   => insert_bigquery(metadata,   &warehouse_config["bigquery"]).await,
        "clickhouse" => insert_clickhouse(metadata, &warehouse_config["clickhouse"]).await,
        "databricks" => insert_databricks(metadata, &warehouse_config["databricks"]).await,
        "redshift"   => insert_redshift(metadata,   &warehouse_config["redshift"]).await,
        "none" | ""  => Ok(()),
        other => Err(format!("Unknown warehouse provider: {other}")),
    }
}
