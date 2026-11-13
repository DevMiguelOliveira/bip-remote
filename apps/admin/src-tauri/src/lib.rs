#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_sessions,
            approve_session,
            end_session,
        ])
        .run(tauri::generate_context!())
        .expect("error while running BIPRemote Admin");
}

#[tauri::command]
async fn list_sessions() -> Result<SessionListResponse, String> {
    // In a real implementation, this would call the backend API
    // For now, return mock data for UI development
    Ok(SessionListResponse {
        sessions: vec![
            Session {
                id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
                host_fingerprint: "a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2".to_string(),
                client_identifier: "admin-user-1".to_string(),
                state: "active".to_string(),
                transport: "direct_p2p".to_string(),
                authorized_at: Some("2024-01-15T10:30:00Z".to_string()),
                approved_at: Some("2024-01-15T10:30:05Z".to_string()),
                started_at: Some("2024-01-15T10:30:10Z".to_string()),
                ended_at: None,
                duration_seconds: Some(3600),
                created_at: "2024-01-15T10:29:55Z".to_string(),
            },
            Session {
                id: "660e8400-e29b-41d4-a716-446655440001".to_string(),
                host_fingerprint: "b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c".to_string(),
                client_identifier: "admin-user-2".to_string(),
                state: "pending".to_string(),
                transport: "relayed".to_string(),
                authorized_at: Some("2024-01-15T11:00:00Z".to_string()),
                approved_at: None,
                started_at: None,
                ended_at: None,
                duration_seconds: None,
                created_at: "2024-01-15T10:59:50Z".to_string(),
            },
            Session {
                id: "770e8400-e29b-41d4-a716-446655440002".to_string(),
                host_fingerprint: "c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2c3d".to_string(),
                client_identifier: "admin-user-3".to_string(),
                state: "ended".to_string(),
                transport: "tcp_hole_punch".to_string(),
                authorized_at: Some("2024-01-15T09:00:00Z".to_string()),
                approved_at: Some("2024-01-15T09:00:05Z".to_string()),
                started_at: Some("2024-01-15T09:00:10Z".to_string()),
                ended_at: Some("2024-01-15T10:30:10Z".to_string()),
                duration_seconds: Some(5400),
                created_at: "2024-01-15T08:59:55Z".to_string(),
            },
        ]
    })
}

#[tauri::command]
async fn approve_session(session_id: String) -> Result<(), String> {
    // In a real implementation, this would call the backend API
    println!("Approving session: {}", session_id);
    Ok(())
}

#[tauri::command]
async fn end_session(session_id: String) -> Result<(), String> {
    // In a real implementation, this would call the backend API
    println!("Ending session: {}", session_id);
    Ok(())
}

// Response types
#[derive(serde::Serialize)]
struct SessionListResponse {
    sessions: Vec<Session>,
}

#[derive(serde::Serialize)]
struct Session {
    id: String,
    host_fingerprint: String,
    client_identifier: String,
    state: String,
    transport: String,
    authorized_at: Option<String>,
    approved_at: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    duration_seconds: Option<i64>,
    created_at: String,
}