use std::sync::Arc;
use tokio::sync::Mutex;
use hyper::{Body, Request, Response, StatusCode};
use crate::authenticator::Authenticator;

/// Handle incoming IPC requests from the admin console
pub async fn handle_request(
    req: Request<Body>,
    _stream: TcpStream,
    auth: Arc<Authenticator>
) -> Result<Response<Body>, hyper::Error> {
    use hyper::Method;
    use hyper::body::Bytes;

    let path = req.uri().path();
    let method = req.method();

    trace!("IPC request: {} {}", method, path);

    match (method, path) {
        // Session management endpoints
        (&Method::GET, "/api/sessions") => {
            // List sessions for this operator
            Ok(Response::new(Body::from(r#"{"sessions": []}"#.into())))
        }
        (&Method::POST, "/api/sessions/approve") => {
            // Approve a remote session
            let body = hyper::body::to_string(req.into_body()).await.un_or_default();
            // TODO: Parse body and approve session
            Ok(Response::new(Body::from(r#"{"status": "approved"}"#.into())))
        }
        (&Method::POST, "/api/sessions/end") => {
            // End an active session
            Ok(Response::new(Body::from(r#"{"status": "ended"}"#.into())))
        }

        // Authentication endpoints
        (&Method::POST, "/api/auth/challenge") => {
            handle_auth_challenge(auth).await
        }

        (&Method::POST, "/api/auth/verify") => {
            // Verify authentication token
            Ok(Response::new(Body::from(r#"{"status": "verified"}"#.into())))
        }

        // Stream control endpoints
        (&Method::POST, "/api/stream/start") => {
            Ok(Response::new(Body::from(r#"{"status": "started"}"#.into())))
        }
        (&Method::POST, "/api/stream/stop") => {
            Ok(Response::new(Body::from(r#"{"status": "stopped"}"#.into())))
        }

        // Control input (keyboard/mouse)
        (&Method::POST, "/api/control/input") => {
            Ok(Response::new(Body::from(r#"{"status": "input_accepted"}"#.into())))
        }

        // Default 404
        _ => {
            let not_found = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("Not Found"))
                .map_err(|e| {
                    warn!("Error building response: {}", e);
                    e
                })?;
            Ok(not_found)
        }
    }
}

/// Handle authentication challenge request
async fn handle_auth_challenge(
    auth: Arc<Authenticator>
) -> Result<Response<Body>, hyper::Error> {
    // Generate challenge and return to client
    let challenge = auth.generate_challenge().await;
    let json = serde_json::to_string(&challenge).unwrap_or_else(|_| r#"{"challenge_id": "fallback"}"#.to_string());

    Ok(Response::new(Body::from(json)))
}