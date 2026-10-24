use std::collections::HashMap;
use tracing::{debug, error, info};
use zbus::zvariant::Value;
use zbus::Connection;

pub const ACTION_REDUCE: &str = "org.memreduct.reduce";
pub const POLKIT_BUS_NAME: &str = "org.freedesktop.PolicyKit1";
pub const POLKIT_OBJECT_PATH: &str = "/org/freedesktop/PolicyKit1/Authority";
pub const POLKIT_INTERFACE: &str = "org.freedesktop.PolicyKit1.Authority";

// Flag 1 = CheckAuthorizationFlags.AllowUserInteraction
const FLAG_ALLOW_USER_INTERACTION: u32 = 1;

pub async fn check_polkit_auth(
    conn: &Connection,
    sender: &str,
    action_id: &str,
    details: HashMap<&str, &str>,
) -> Result<bool, zbus::Error> {
    debug!("Checking Polkit auth for sender={sender}, action={action_id}");

    let mut subject_details: HashMap<String, Value> = HashMap::new();
    subject_details.insert("name".to_string(), Value::from(sender));
    let subject = ("system-bus-name", subject_details);

    let polkit_details: HashMap<String, String> = details
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    let proxy =
        zbus::Proxy::new(conn, POLKIT_BUS_NAME, POLKIT_OBJECT_PATH, POLKIT_INTERFACE).await?;

    let reply: (bool, bool, HashMap<String, String>) = proxy
        .call(
            "CheckAuthorization",
            &(
                subject,
                action_id,
                polkit_details,
                FLAG_ALLOW_USER_INTERACTION,
                "", // cancellation_id
            ),
        )
        .await
        .map_err(|e| {
            error!("Failed to call Polkit CheckAuthorization: {e}");
            e
        })?;

    let (is_authorized, is_challenge, _details) = reply;
    info!(
        "Polkit auth result for sender {sender}: authorized={is_authorized}, challenge={is_challenge}"
    );

    Ok(is_authorized)
}
