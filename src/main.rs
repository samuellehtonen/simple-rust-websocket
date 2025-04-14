use std::{
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
    time::Duration,
};

use futures::{SinkExt, StreamExt};
use tokio::{fs, sync::broadcast, time};
use warp::Filter;
use warp::ws::{Message, WebSocket};

#[tokio::main]
async fn main() {
    let (tx, _) = broadcast::channel::<String>(16);
    let connection_count = Arc::new(AtomicUsize::new(0));
    // Clone for the async tokio spawn task
    let connection_count_clone = Arc::clone(&connection_count);

    // Spawn a task to log active connections every minute
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let count = connection_count_clone.load(std::sync::atomic::Ordering::SeqCst);
            println!("Active WebSocket connections: {}", count);
        }
    });

    // Poll file for changes
    let tx_poll = tx.clone();
    tokio::spawn(async move {
        let mut last_content = String::new();
        // Poll file changes every 2 seconds
        let mut interval = time::interval(Duration::from_secs(2));

        loop {
            interval.tick().await;
            // Read file to string and do simple string comparison, it's fast enough for our small file
            match fs::read_to_string("/full/path/to/small_data_file.csv").await {
                Ok(content) => {
                    if content != last_content {
                        if tx_poll.send(content.clone()).is_err() {
                            eprintln!("Broadcast channel overflow!");
                        }
                        last_content = content;
                    }
                }
                Err(err) => eprintln!("Failed to read file: {:?}", err),
            }
        }
    });

    // Filters
    let tx_filter = warp::any().map(move || tx.clone());
    let count_filter = warp::any().map({
        let count = Arc::clone(&connection_count);
        move || Arc::clone(&count)
    });

    // WebSocket route
    let ws_route = warp::path("ws")
        .and(warp::ws())
        .and(tx_filter)
        .and(count_filter)
        .map(|ws: warp::ws::Ws, tx, count| {
            ws.on_upgrade(move |socket| handle_socket(socket, tx, count))
        });

    println!("Server running on ws://localhost:8080/ws");
    warp::serve(ws_route).run(([0, 0, 0, 0], 8080)).await;
}

pub async fn handle_socket(
    ws: WebSocket,
    tx: broadcast::Sender<String>,
    connection_count: Arc<AtomicUsize>,
) {
    let mut rx = tx.subscribe();
    let (mut ws_tx, mut ws_rx) = ws.split();

    // Increment connection count
    connection_count.fetch_add(1, Ordering::SeqCst);

    tokio::spawn(async move {
        // Use select! to handle both outgoing and incoming streams
        tokio::select! {
            // Sending loop: pushes messages from broadcast to WebSocket
            _ = async {
                while let Ok(content) = rx.recv().await {
                    if ws_tx.send(Message::text(content)).await.is_err() {
                        break; // client likely disconnected
                    }
                }
            } => {},

            // Receiving loop: watches for disconnects (like close frame)
            _ = async {
                while let Some(result) = ws_rx.next().await {
                    match result {
                        Ok(msg) => {
                            if msg.is_close() {
                                break;
                            }
                        }
                        Err(_) => break, // client disconnected unexpectedly
                    }
                }
            } => {},
        }

        // Decrement connection count
        connection_count.fetch_sub(1, Ordering::SeqCst);
    });
}
