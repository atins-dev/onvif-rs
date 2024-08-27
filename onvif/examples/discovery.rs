extern crate onvif;
use std::sync::{atomic::AtomicBool, Arc};

use onvif::discovery;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    use futures_util::stream::StreamExt;
    const MAX_CONCURRENT_JUMPERS: usize = 100;

    discovery::discover(
        std::time::Duration::from_secs(3),
        Arc::new(AtomicBool::new(false)),
    )
    .await
    .unwrap()
    .for_each_concurrent(MAX_CONCURRENT_JUMPERS, |addr| async move {
        println!("Device found: {addr:?}");
    })
    .await;
}
