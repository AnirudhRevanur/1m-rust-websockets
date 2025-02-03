use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use std::fs::OpenOptions;
use std::io::Write;
use tokio::task;
use tokio::time::{self, Duration};
use tokio_tungstenite::connect_async;
use url::Url;

const TIME_PERIOD_MINUTES: u64 = 1;
const NUM_CLIENTS: u64 = 100;
const SERVER_ADDRESS: &str = "ws://localhost:8080/ws";

#[tokio::main]
async fn main() {
    let total_time_ms = TIME_PERIOD_MINUTES * 60 * 1000;

    let interval_ms = total_time_ms / NUM_CLIENTS;

    let mut interval = time::interval(Duration::from_millis(interval_ms));

    for i in 0..NUM_CLIENTS {
        interval.tick().await;

        let client_id = i;
        task::spawn(async move {
            send_message(client_id).await;
        });
    }
}

async fn send_message(client_id: u64) {
    let url = Url::parse(SERVER_ADDRESS).expect("Invalid WebSocket URL");
    let url_string = url.as_str();
    let (ws_stream, _) = match connect_async(url_string).await {
        Ok(connection) => connection,
        Err(e) => {
            eprintln!("Client {}: Error connecting to websocket: {}", client_id, e);
            return;
        }
    };

    let (mut write, _read) = ws_stream.split();

    let client_id_str = format!("{}", client_id);
    if let Err(e) = write.send(client_id_str.clone().into()).await {
        eprintln!("Client {}: Error sending client ID: {}", client_id, e);
        return;
    }

    let payload = "A".repeat(512_000);
    let message = format!("Client {} - {}", client_id, payload);

    if let Err(e) = write.send(message.clone().into()).await {
        eprintln!("Client {}: Error sending message: {}", client_id, e);
        return;
    }

    println!("Client {}: Message sent", client_id);
}

fn save_to_file(filename: &str, data: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename)?;

    writeln!(file, "{}", data)?;
    Ok(())
}
