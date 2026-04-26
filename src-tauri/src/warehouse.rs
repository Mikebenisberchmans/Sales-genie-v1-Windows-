use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

type HmacSha256 = Hmac<sha2::Sha256>;

fn hmac_sign(key: &[u8], msg: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

// ---------------------------------------------------------------------------
// Snowflake — SQL API v2
// ---------------------------------------------------------------------------
async fn insert_snowflake(metadata: &Value, cfg: &Value) -> Result<(), String> {
    let client = Client::new();
    let account = cfg["account"].as_str().unwrap_or("");
    let username = cfg["username"].as_str().unwrap_or("");
    let password = cfg["password"].as_str().unwrap_or("");
    let database = cfg["database"].as_str().unwrap_or("");
    let schema = cfg["schema"].as_str().unwrap_or("");
    let warehouse = cfg["warehouse"].as_str().unwrap_or("");
    let table = cfg["table"].as_str().unwrap_or("genie_recordings");

    let url = format!("https://{account}.snowflakecomputing.com/api/v2/statements");
    let creds = STANDARD.encode(format!("{username}:{password}"));

    let stmt = format!(
        "INSERT INTO {database}.{schema}.{table} \
         (opp_id, submission_date, duration_seconds, salesperson_name, salesperson_id, \
          mic_url, sys_url, mic_local_path, sys_local_path, sample_rate, channels) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    );

    let body = json!({
        "statement": stmt,
        "warehouse": warehouse,
        "bindings": {
            "1":  { "type": "TEXT",    "value": metadata["opp_id"] },
            "2":  { "type": "TEXT",    "value": metadata["submission_date"] },
            "3":  { "type": "FIXED",   "value": metadata["duration_seconds"].to_string() },
            "4":  { "type": "TEXT",    "value": metadata["salesperson_name"] },
            "5":  { "type": "TEXT",    "value": metadata["salesperson_id"] },
            "6":  { "type": "TEXT",    "value": metadata["mic_url"] },
            "7":  { "type": "TEXT",    "value": metadata["sys_url"] },
            "8":  { "type": "TEXT",    "value": metadata["mic_local_path"] },
            "9":  { "type": "TEXT",    "value": metadata["sys_local_path"] },
            "10": { "type": "FIXED",   "value": metadata["sample_rate"].to_string() },
            "11": { "type": "FIXED",   "value": metadata["channels"].to_string() }
        }
    });

    client
        .post(&url)
        .header("authorization", format!("Basic {creds}"))
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .header("X-Snowflake-Authorization-Token-Type", "BASIC")
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

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
