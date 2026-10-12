use dotenvy::dotenv;
use rust_grpc::inventory::purchase_repository::{PurchaseError, PurchaseRepository};
use rust_grpc::inventory::repository::ProductRepository;
use rust_grpc::inventory_v1::inventory_server::{Inventory, InventoryServer};
use rust_grpc::inventory_v1::{
    CreatePurchaseRequest, CreatePurchaseResponse, GetProductRequest, GetProductResponse,
    PurchaseEvent, WatchPurchasesRequest,
};
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use std::env;
use std::error::Error;
use std::pin::Pin;
use std::str::FromStr;
use tokio::sync::{broadcast, watch};
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tonic::transport::{Certificate, Identity, Server, ServerTlsConfig};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

const DATABASE_URL: &str = "sqlite://data/stockguard.db";
const GRPC_ADDRESS: &str = "[::1]:50051";
const CA_CERT_PATH: &str = "certs/ca.pem";
const SERVER_CERT_PATH: &str = "certs/inventory.pem";
const SERVER_KEY_PATH: &str = "certs/inventory-key.pem";
const EVENT_CHANNEL_CAPACITY: usize = 128;

struct InventoryService {
    product_repository: ProductRepository,
    purchase_repository: PurchaseRepository,
    purchase_events: broadcast::Sender<PurchaseEvent>,
    shutdown: watch::Receiver<bool>,
}

impl InventoryService {
    fn new(
        pool: SqlitePool,
        purchase_events: broadcast::Sender<PurchaseEvent>,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            product_repository: ProductRepository::new(pool.clone()),
            purchase_repository: PurchaseRepository::new(pool),
            purchase_events,
            shutdown,
        }
    }
}

#[tonic::async_trait]
impl Inventory for InventoryService {
    async fn get_product(
        &self,
        request: Request<GetProductRequest>,
    ) -> Result<Response<GetProductResponse>, Status> {
        let product_id = request.into_inner().product_id;

        let product = self
            .product_repository
            .get_by_id(product_id)
            .await
            .map_err(|error| {
                error!(product_id, %error, "failed to read product");
                Status::internal("inventory database failure")
            })?;

        match product {
            Some(product) => {
                debug!(product_id, "product retrieved");
                Ok(Response::new(GetProductResponse {
                    product_id: product.id,
                    name: product.name,
                    stock: product.stock,
                }))
            }
            None => {
                info!(product_id, "product not found");
                Err(Status::not_found("product not found"))
            }
        }
    }

    async fn create_purchase(
        &self,
        request: Request<CreatePurchaseRequest>,
    ) -> Result<Response<CreatePurchaseResponse>, Status> {
        let request = request.into_inner();
        let purchase = self
            .purchase_repository
            .create(request.product_id, request.quantity)
            .await
            .map_err(|error| match error {
                PurchaseError::InvalidQuantity => {
                    info!(
                        product_id = request.product_id,
                        quantity = request.quantity,
                        "purchase rejected: invalid quantity"
                    );
                    Status::invalid_argument("quantity must be greater than zero")
                }
                PurchaseError::ProductNotFound => {
                    info!(
                        product_id = request.product_id,
                        "purchase rejected: product not found"
                    );
                    Status::not_found("product not found")
                }
                PurchaseError::InsufficientStock => {
                    info!(
                        product_id = request.product_id,
                        quantity = request.quantity,
                        "purchase rejected: insufficient stock"
                    );
                    Status::failed_precondition("insufficient stock")
                }
                PurchaseError::Database(error) => {
                    error!(
                        product_id = request.product_id,
                        %error,
                        "purchase database failure"
                    );
                    Status::internal("inventory database failure")
                }
                PurchaseError::Timestamp(error) => {
                    error!(%error, "purchase timestamp failure");
                    Status::internal("inventory timestamp failure")
                }
            })?;

        let event = PurchaseEvent {
            purchase_id: purchase.id,
            product_id: purchase.product_id,
            quantity: purchase.purchased_quantity,
            purchased_at: purchase.purchased_at.clone(),
        };

        if self.purchase_events.send(event).is_err() {
            debug!(
                purchase_id = purchase.id,
                "purchase committed with no active event subscribers"
            );
        }

        info!(
            purchase_id = purchase.id,
            product_id = purchase.product_id,
            quantity = purchase.purchased_quantity,
            remaining_stock = purchase.remaining_stock,
            "purchase committed"
        );

        Ok(Response::new(CreatePurchaseResponse {
            purchase_id: purchase.id,
            product_id: purchase.product_id,
            purchased_quantity: purchase.purchased_quantity,
            remaining_stock: purchase.remaining_stock,
            purchased_at: purchase.purchased_at,
        }))
    }

    type WatchPurchasesStream =
        Pin<Box<dyn Stream<Item = Result<PurchaseEvent, Status>> + Send + 'static>>;

    async fn watch_purchases(
        &self,
        _request: Request<WatchPurchasesRequest>,
    ) -> Result<Response<Self::WatchPurchasesStream>, Status> {
        let receiver = self.purchase_events.subscribe();
        let shutdown = self.shutdown.clone();
        info!("purchase event subscriber connected");

        let events = BroadcastStream::new(receiver).map(|result| {
            Some(match result {
                Ok(event) => Ok(event),
                Err(error) => {
                    warn!(%error, "purchase event subscriber lagged");
                    Err(Status::resource_exhausted(
                        "purchase event subscriber lagged",
                    ))
                }
            })
        });
        let shutdown = WatchStream::new(shutdown)
            .filter_map(|shutdown_requested| shutdown_requested.then_some(None));
        let stream = events
            .merge(shutdown)
            .take_while(Option::is_some)
            .filter_map(|item| item);

        Ok(Response::new(Box::pin(stream)))
    }
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn load_server_tls() -> Result<ServerTlsConfig, Box<dyn Error>> {
    let ca_path = env::var("TLS_CA_CERT").unwrap_or_else(|_| CA_CERT_PATH.to_string());
    let cert_path = env::var("INVENTORY_TLS_CERT").unwrap_or_else(|_| SERVER_CERT_PATH.to_string());
    let key_path = env::var("INVENTORY_TLS_KEY").unwrap_or_else(|_| SERVER_KEY_PATH.to_string());

    let ca = std::fs::read(ca_path)?;
    let cert = std::fs::read(cert_path)?;
    let key = std::fs::read(key_path)?;

    Ok(ServerTlsConfig::new()
        .identity(Identity::from_pem(cert, key))
        .client_ca_root(Certificate::from_pem(ca)))
}

async fn shutdown_signal(shutdown: watch::Sender<bool>) {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler should install");
        tokio::select! {
            result = tokio::signal::ctrl_c() => {
                if let Err(error) = result {
                    error!(%error, "failed to listen for Ctrl+C");
                }
            }
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        error!(%error, "failed to listen for Ctrl+C");
    }

    info!("inventory shutdown requested");
    let _ = shutdown.send(true);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv().ok();
    init_tracing();

    let db_url = env::var("DATABASE_URL").unwrap_or_else(|_| DATABASE_URL.to_string());
    let address = env::var("INVENTORY_GRPC_ADDRESS").unwrap_or_else(|_| GRPC_ADDRESS.to_string());
    let address = address.parse()?;

    let connection_options = SqliteConnectOptions::from_str(&db_url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePool::connect_with(connection_options).await?;
    sqlx::migrate!().run(&pool).await?;
    info!("database migrations applied");

    let (purchase_events, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let service = InventoryService::new(pool, purchase_events, shutdown_rx);
    let tls = load_server_tls()?;

    info!(%address, "inventory gRPC server listening with mTLS");
    Server::builder()
        .tls_config(tls)?
        .add_service(InventoryServer::new(service))
        .serve_with_shutdown(address, shutdown_signal(shutdown_tx))
        .await?;

    info!("inventory server stopped");
    Ok(())
}
