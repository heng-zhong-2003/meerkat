use tokio::sync::mpsc;

pub async fn defworker_proc(mut receive_channel: mpsc::Receiver<i32>) {
    println!("enter defwroker");
    while let Some(val) = receive_channel.recv().await {
        println!("got {}", val);
    }
}
