use super::models::PurchaseNotification;
use crate::inventory_v1::WatchPurchasesRequest;
use crate::inventory_v1::inventory_client::InventoryClient;
use std::time::Duration;
use tokio::sync::{broadcast, watch};
use tonic::Request;
use tonic::transport::Channel;
use tracing::{debug, info, warn};

const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(5);

pub async fn forward_purchase_events(
    mut client: InventoryClient<Channel>,
    public_events: broadcast::Sender<PurchaseNotification>,
    mut shutdown: watch::Receiver<bool>,
) {
    let mut retry_delay = INITIAL_RETRY_DELAY;

    loop {
        if *shutdown.borrow() {
            break;
        }

        info!("connecting to Inventory purchase stream");
        let response = tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
                continue;
            }
            response = client.watch_purchases(Request::new(WatchPurchasesRequest {})) => response,
        };

        match response {
            Ok(response) => {
                info!("connected to Inventory purchase stream");
                retry_delay = INITIAL_RETRY_DELAY;
                let mut stream = response.into_inner();

                loop {
                    let message = tokio::select! {
                        changed = shutdown.changed() => {
                            if changed.is_err() || *shutdown.borrow() {
                                info!("purchase stream subscriber stopping");
                                return;
                            }
                            continue;
                        }
                        message = stream.message() => message,
                    };

                    match message {
                        Ok(Some(event)) => {
                            let purchase_id = event.purchase_id;
                            let notification = PurchaseNotification {
                                purchase_id,
                                product_id: event.product_id,
                                quantity: event.quantity,
                                purchased_at: event.purchased_at,
                            };

                            if public_events.send(notification).is_err() {
                                debug!(purchase_id, "purchase event received without SSE clients");
                            } else {
                                info!(purchase_id, "purchase event forwarded to SSE clients");
                            }
                        }
                        Ok(None) => {
                            warn!("Inventory purchase stream ended");
                            break;
                        }
                        Err(error) => {
                            warn!(%error, "Inventory purchase stream failed");
                            break;
                        }
                    }
                }
            }
            Err(error) => {
                warn!(%error, "failed to establish Inventory purchase stream");
            }
        }

        info!(
            delay_seconds = retry_delay.as_secs(),
            "retrying purchase stream"
        );
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            _ = tokio::time::sleep(retry_delay) => {}
        }
        retry_delay = std::cmp::min(retry_delay * 2, MAX_RETRY_DELAY);
    }

    info!("purchase stream subscriber stopped");
}
