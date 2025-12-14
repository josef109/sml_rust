use crate::model::SharedAppState;
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        Html, IntoResponse,
    },
    routing::get,
    Router,
};
use tokio::{fs::File, io::AsyncReadExt};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::services::ServeDir;
use tracing::info;

// --- 5. Handler für die HTML-Seite (Liest index.html aus dem static-Ordner) ---

async fn html_handler() -> impl IntoResponse {
    let mut file = match File::open("static/index.html").await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Fehler beim Lesen von index.html: {}", e);
            // Im Fehlerfall eine einfache Fehlermeldung zurückgeben
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(
                    "<h1>500 Internal Server Error</h1><p>Konnte index.html nicht laden.</p>"
                        .to_string(),
                ),
            );
        }
    };

    let mut contents = String::new();
    if let Err(e) = file.read_to_string(&mut contents).await {
        eprintln!("Fehler beim Lesen des Inhalts von index.html: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html("<h1>500 Internal Server Error</h1><p>Konnte Inhalt nicht lesen.</p>".to_string()),
        );
    }

    (StatusCode::OK, Html(contents))
}

pub async fn start_server(
    addr: &str,
    image_path: &str,
    shared_state: SharedAppState,
) -> anyhow::Result<()> {
    // 1. Service für das Standard-Static-Verzeichnis ('static')
    let static_files_service = ServeDir::new("static");

    // 2. NEUER Service für den dynamisch übergebenen Bildpfad
    let image_service = ServeDir::new(image_path);

    let app = Router::new()
        // Hauptroute (liest index.html)
        .route("/", get(html_handler))
        // SSE-Route
        .route("/events", get(sse_handler))
        // Favicon Route (zur Vermeidung des 404-Fehlers)
        .route("/favicon.ico", get(|| async { StatusCode::NO_CONTENT }))
        // Service für statische Dateien (CSS, JS, Bilder etc.)
        .nest_service("/static", static_files_service)
        // Alle Anfragen an /images/... werden an das Verzeichnis im image_path weitergeleitet
        .nest_service("/images", image_service)
        .with_state(shared_state); // Hinzufügen der State-Weitergabe
                                   //       .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server läuft auf {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn sse_handler(
    State(state): State<SharedAppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, axum::Error>>> {
    let rx = {
        let s = state.lock().unwrap();
        s.tx.subscribe()
    };

    let stream = BroadcastStream::new(rx).map(|msg| match msg {
        Ok(data) => {
            let json = serde_json::to_string(&data).unwrap_or_default();
            Ok(Event::default().data(json))
        }
        Err(_) => Ok(Event::default()),
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}
