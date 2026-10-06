use async_graphql::{Context, Result, Subscription};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::{Stream, StreamExt};

use crate::auth::LoggedIn;
use crate::models::Book;

pub struct Subscription;

#[Subscription]
impl Subscription {
    // Needs a logged-in user. Over a WebSocket the token is sent when the
    // connection opens, not per subscription.
    #[graphql(guard = "LoggedIn")]
    async fn book_added(&self, ctx: &Context<'_>) -> Result<impl Stream<Item = Book> + use<>> {
        let events = ctx.data::<broadcast::Sender<Book>>()?;
        let stream = BroadcastStream::new(events.subscribe());
        Ok(stream.filter_map(|event| event.ok()))
    }
}