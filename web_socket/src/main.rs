use futures_util::{SinkExt, StreamExt};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use warp::Filter;

#[tokio::main]
async fn main() {
    let message_counter = Arc::new(TokioMutex::new(0));

    let websocket_route = warp::path("ws")
        .and(warp::ws())
        .and(with_counter(message_counter.clone()))
        .map(|ws: warp::ws::Ws, counter| {
            ws.on_upgrade(move |socket| handle_connection(socket, counter))
        });

    let routes = websocket_route.with(warp::cors().allow_any_origin());

    println!("WebSocket server started at ws://localhost:8080/ws");
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}

fn with_counter(
    counter: Arc<TokioMutex<u32>>,
) -> impl Filter<Extract = (Arc<TokioMutex<u32>>,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || counter.clone())
}

async fn handle_connection(ws: warp::ws::WebSocket, counter: Arc<TokioMutex<u32>>) {
    let (mut write, mut read) = ws.split();

    let client_id = match read.next().await {
        Some(Ok(msg)) => msg.to_str().unwrap_or("unknown_client").to_string(),
        _ => {
            eprintln!("Failed to read client ID");
            return;
        }
    };

    println!("Client connected: {}", client_id);

    while let Some(result) = read.next().await {
        match result {
            Ok(msg) => {
                let mut count = counter.lock().await;
                *count += 1;

                let message_text = msg.to_str().unwrap_or("Invalid UTF-8").to_string();
                println!("Message #{} from {}: {}", count, client_id, message_text);

                if let Err(e) = save_to_file(&client_id, &message_text) {
                    eprintln!("Error saving message for client {}: {}", client_id, e);
                }

                let response = format!("Message #{} received by the server", count);
                if write.send(warp::ws::Message::text(response)).await.is_err() {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Error reading message: {}", e);
                break;
            }
        }
    }
}

fn get_file_path(client_id: &str) -> PathBuf {
    let temp_dir = std::env::temp_dir();
    println!("{:?}", temp_dir);
    temp_dir.join(format!("client_{}.txt", client_id)) // Name file after client ID
}

fn save_to_file(client_id: &str, data: &str) -> std::io::Result<()> {
    let file_path = get_file_path(client_id);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)?;

    writeln!(file, "{}", data)?;
    println!("Message saved to: {:?}", file_path);
    Ok(())
}
