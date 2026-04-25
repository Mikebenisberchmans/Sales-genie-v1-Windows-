use base64::{engine::general_purpose::STANDARD, Engine};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde_json::Value;
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<sha2::Sha256>;

fn hmac_sign(key: &[u8], msg: &str) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key error");
    mac.update(msg.as_bytes());
    mac.finalize().into_bytes().to_vec()
}

async fn upload_s3(
    client: &Client,
    bucket: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
    prefix: &str,
    object_key: &str,
    body: Vec<u8>,
) -> Result<String, String> {
    let host = format!("{bucket}.s3.{region}.amazonaws.com");
    let full_key = if prefix.is_empty() {
        object_key.to_string()
    } else {
        format!("{prefix}/{object_key}")
    };
    let url = format!("https://{host}/{full_key}");

    let now = chrono::Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();

    let payload_hash = hex::encode(Sha256::digest(&body));

    let canonical_headers = format!(
        "content-type:audio/wav\nhost:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n"
    );
    let signed_headers = "content-type;host;x-amz-content-sha256;x-amz-date";

    let canonical_request = format!(
        "PUT\n/{full_key}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );

    let cr_hash = hex::encode(Sha256::digest(canonical_request.as_bytes()));
    let scope = format!("{date_stamp}/{region}/s3/aws4_request");
    let string_to_sign = format!("AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{cr_hash}");

    let k_secret = format!("AWS4{secret_key}");
    let k_date = hmac_sign(k_secret.as_bytes(), &date_stamp);
    let k_region = hmac_sign(&k_date, region);
    let k_service = hmac_sign(&k_region, "s3");
    let k_signing = hmac_sign(&k_service, "aws4_request");

    let signature = hex::encode({
        let mut mac = HmacSha256::new_from_slice(&k_signing).expect("HMAC key error");
        mac.update(string_to_sign.as_bytes());
        mac.finalize().into_bytes()
    });

    let auth = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed_headers}, Signature={signature}"
    );

    client
        .put(&url)
        .header("host", &host)
        .header("x-amz-date", &amz_date)
        .header("x-amz-content-sha256", &payload_hash)
        .header("content-type", "audio/wav")
        .header("authorization", &auth)
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(format!("https://{host}/{full_key}"))
}

async fn upload_azure(
    client: &Client,
    account: &str,
    account_key: &str,
    container: &str,
    blob_name: &str,
    body: Vec<u8>,
) -> Result<String, String> {
    let url = format!("https://{account}.blob.core.windows.net/{container}/{blob_name}");

    let now = chrono::Utc::now()
        .format("%a, %d %b %Y %H:%M:%S GMT")
        .to_string();
    let content_length = body.len().to_string();

    let string_to_sign = format!(
        "PUT\n\naudio/wav\n{content_length}\n\nx-ms-blob-type:BlockBlob\nx-ms-date:{now}\n/{account}/{container}/{blob_name}"
    );

    let key_bytes = STANDARD
        .decode(account_key)
        .map_err(|e| format!("Azure key decode error: {e}"))?;

    let mut mac = HmacSha256::new_from_slice(&key_bytes)
        .map_err(|e| format!("HMAC init error: {e}"))?;
    mac.update(string_to_sign.as_bytes());
    let sig = STANDARD.encode(mac.finalize().into_bytes());

    let auth = format!("SharedKey {account}:{sig}");

    client
        .put(&url)
        .header("x-ms-date", &now)
        .header("x-ms-version", "2020-10-02")
        .header("x-ms-blob-type", "BlockBlob")
        .header("content-type", "audio/wav")
        .header("content-length", &content_length)
        .header("authorization", &auth)
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    Ok(url)
}

pub async fn upload(
    mic_path: &str,
    sys_path: &str,
    opp_id: &str,
    config: &Value,
) -> Result<Value, String> {
    let client = Client::new();
    let provider = config["objectStore"]["provider"]
        .as_str()
        .unwrap_or("none");

    let mic_bytes = std::fs::read(mic_path).map_err(|e| format!("Read mic file: {e}"))?;
    let sys_bytes = std::fs::read(sys_path).map_err(|e| format!("Read sys file: {e}"))?;

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let mic_key = format!("{opp_id}/mic_{ts}.wav");
    let sys_key = format!("{opp_id}/sys_{ts}.wav");

    let (mic_url, sys_url) = match provider {
        "s3" => {
            let s3 = &config["objectStore"]["s3"];
            let bucket = s3["bucket"].as_str().unwrap_or("");
            let region = s3["region"].as_str().unwrap_or("us-east-1");
            let access_key = s3["accessKeyId"].as_str().unwrap_or("");
            let secret_key = s3["secretAccessKey"].as_str().unwrap_or("");
            let prefix = s3["prefix"].as_str().unwrap_or("");

            let mic_url = upload_s3(
                &client, bucket, region, access_key, secret_key, prefix, &mic_key, mic_bytes,
            )
            .await?;
            let sys_url = upload_s3(
                &client, bucket, region, access_key, secret_key, prefix, &sys_key, sys_bytes,
            )
            .await?;
            (mic_url, sys_url)
        }
        "azure" => {
            let az = &config["objectStore"]["azure"];
            let account = az["accountName"].as_str().unwrap_or("");
            let account_key = az["accountKey"].as_str().unwrap_or("");
            let container = az["containerName"].as_str().unwrap_or("");

            let mic_url =
                upload_azure(&client, account, account_key, container, &mic_key, mic_bytes).await?;
            let sys_url =
                upload_azure(&client, account, account_key, container, &sys_key, sys_bytes).await?;
            (mic_url, sys_url)
        }
        "gcs" => {
            return Err("GCS upload not yet implemented — use S3 or Azure Blob".into());
        }
        other => {
            return Err(format!("Unknown object store provider: {other}"));
        }
    };

    Ok(serde_json::json!({ "mic_url": mic_url, "sys_url": sys_url }))
}
