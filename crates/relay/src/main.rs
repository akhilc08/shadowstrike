use tokio::net::TcpListener;
use tokio::signal;
use tracing::info;

use relay::room;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port = std::env::var("PORT").unwrap_or_else(|_| "9001".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.expect("Failed to bind");
    info!("Relay server listening on {}", addr);

    let rooms = room::new_room_map();

    loop {
        tokio::select! {
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, addr)) => {
                        let rooms = rooms.clone();
                        tokio::spawn(relay::server::handle_connection(stream, addr, rooms));
                    }
                    Err(e) => {
                        tracing::warn!("Accept error: {}", e);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                info!("Shutting down relay server");
                break;
            }
        }
    }
}
