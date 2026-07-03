use uuid::{uuid, Uuid};

const OFFLINE_NAMESPACE: Uuid = uuid!("6c3465a3-69cf-4906-b78c-b0b221ea6d8d");

/// Genera el UUID offline estándar de Minecraft (UUID v3 de "OfflinePlayer:<nombre>").
pub fn offline_uuid(username: &str) -> Uuid {
    Uuid::new_v3(&OFFLINE_NAMESPACE, format!("OfflinePlayer:{username}").as_bytes())
}

/// Token ficticio para modo offline.
pub fn offline_access_token() -> &'static str {
    "0"
}
