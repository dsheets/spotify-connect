use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};

use crate::Error;

// see <https://developer.spotify.com/documentation/commercial-hardware/implementation/guides/zeroconf>
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    #[serde(rename = "deviceID")]
    pub device_id: String,
    pub remote_name: String,
    pub public_key: String,
    pub active_user: Option<String>, // undocumented but useful and returned by both librespot and librespot-java
    pub token_type: Option<String>,  // required at least as of 2.9.0
    pub client_id: Option<String>,   // required at least as of 2.9.0
    pub scope: Option<String>,       // required at least as of 2.9.0
}

/// Get the necessary information from the remote device
pub fn get_device_info(base_url: &str) -> Result<DeviceInfo, Error> {
    let response = minreq::get(base_url)
        .with_param("action", "getInfo")
        .send()
        .map_err(|e| Error::CouldNotGetDeviceInfo(String::from(base_url), e.into()))?;

    let device_info = serde_json::from_str(
        response
            .as_str()
            .map_err(|e| Error::CouldNotGetDeviceInfo(String::from(base_url), e.into()))?,
    ).map_err(|e| Error::CouldNotGetDeviceInfo(String::from(base_url), e.into()))?;

    Ok(device_info)
}

/// Authenticate on the remote device thanks to the encrypted blob
pub fn add_user(
    base_url: &str,
    username: &str,
    blob: &str,
    my_public_key: &str,
    token_type: Option<&str>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let device_id = hex::encode(Sha1::digest("spotify-connect".as_bytes()));
    let login_id = hex::encode(rand::thread_rng().gen::<[u8; 16]>());

    let params = [
        ("action", "addUser"),
        ("userName", username),
        ("blob", blob),
        ("clientKey", my_public_key),
        ("deviceId", &device_id),
        ("deviceName", "spotify-connect"),
        ("loginId", &login_id),
    ];
    let mut body = String::with_capacity(1024);
    for (i, (k, v)) in params.iter().enumerate() {
        if i != 0 {
            body.push('&');
        }
        body.push_str(k);
        body.push('=');
        body.push_str(&urlencoding::encode(v));
    }

    if let Some(token_type) = token_type {
        body.push_str("&tokenType=");
        body.push_str(&urlencoding::encode(token_type));
    }

    let request = minreq::post(base_url)
        .with_header("Content-Type", "application/x-www-form-urlencoded")
        .with_body(body);

    let response = request.send()?;

    let v: Value = serde_json::from_str(response.as_str()?)?;

    match v["statusString"].as_str() {
        Some("ERROR-OK") | Some("OK") => Ok(()),
        _ => Err(v.to_string().into()),
    }
}
