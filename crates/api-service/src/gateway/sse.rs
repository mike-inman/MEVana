use std::convert::Infallible;
use std::time::Duration;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::StreamExt;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

pub fn into_sse(
    receiver: broadcast::Receiver<String>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = BroadcastStream::new(receiver).filter_map(|result| async move {
        match result {
            Ok(payload) => Some(Ok(Event::default().data(payload))),
            Err(_) => None,
        }
    });

    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
