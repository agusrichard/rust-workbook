use super::errors::ErrorResponse;
use super::models::{
    CreatePurchaseRequest as PublicPurchaseRequest,
    CreatePurchaseResponse as PublicPurchaseResponse, ProductResponse, PurchaseNotification,
};
use crate::inventory_v1::inventory_client::InventoryClient;
use crate::inventory_v1::{CreatePurchaseRequest, GetProductRequest};
use axum::Json;
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Sse;
use axum::response::sse::{Event, KeepAlive};
use std::convert::Infallible;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio_stream::Stream;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use tonic::Request;
use tonic::transport::Channel;
use tracing::{error, warn};

type ApiError = (StatusCode, Json<ErrorResponse>);
type ApiResult<T> = Result<(StatusCode, Json<T>), ApiError>;

fn api_error(status: StatusCode, message: &str) -> ApiError {
    (status, Json(ErrorResponse::new(message)))
}

fn parse_product_id(value: &str) -> Result<i64, ApiError> {
    value.parse::<i64>().map_err(|_| {
        api_error(
            StatusCode::BAD_REQUEST,
            "product_id must be a valid integer",
        )
    })
}

fn map_inventory_error(status: tonic::Status) -> ApiError {
    match status.code() {
        tonic::Code::InvalidArgument => {
            api_error(StatusCode::BAD_REQUEST, "invalid purchase request")
        }
        tonic::Code::NotFound => api_error(StatusCode::NOT_FOUND, "product not found"),
        tonic::Code::FailedPrecondition => api_error(StatusCode::CONFLICT, "insufficient stock"),
        tonic::Code::DeadlineExceeded => {
            api_error(StatusCode::GATEWAY_TIMEOUT, "inventory request timed out")
        }
        _ => api_error(
            StatusCode::SERVICE_UNAVAILABLE,
            "inventory service is unavailable",
        ),
    }
}

#[derive(Clone)]
pub struct AppState {
    pub inventory_client: InventoryClient<Channel>,
    pub purchase_events: broadcast::Sender<PurchaseNotification>,
}

pub async fn get_product(
    State(state): State<AppState>,
    Path(product_id): Path<String>,
) -> ApiResult<ProductResponse> {
    let product_id = parse_product_id(&product_id)?;
    let mut client = state.inventory_client.clone();
    let mut request = Request::new(GetProductRequest { product_id });
    request.set_timeout(Duration::from_secs(2));

    let product = client
        .get_product(request)
        .await
        .map_err(map_inventory_error)?
        .into_inner();

    Ok((
        StatusCode::OK,
        Json(ProductResponse {
            product_id: product.product_id,
            name: product.name,
            stock: product.stock,
        }),
    ))
}

pub async fn create_purchase(
    State(state): State<AppState>,
    payload: Result<Json<PublicPurchaseRequest>, JsonRejection>,
) -> ApiResult<PublicPurchaseResponse> {
    let Json(payload) =
        payload.map_err(|_| api_error(StatusCode::BAD_REQUEST, "invalid JSON request"))?;

    if payload.quantity <= 0 {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "quantity must be greater than zero",
        ));
    }

    let mut client = state.inventory_client.clone();
    let mut request = Request::new(CreatePurchaseRequest {
        product_id: payload.product_id,
        quantity: payload.quantity,
    });
    request.set_timeout(Duration::from_secs(2));

    let purchase = client
        .create_purchase(request)
        .await
        .map_err(map_inventory_error)?
        .into_inner();

    Ok((
        StatusCode::CREATED,
        Json(PublicPurchaseResponse {
            purchase_id: purchase.purchase_id,
            product_id: purchase.product_id,
            purchased_quantity: purchase.purchased_quantity,
            remaining_stock: purchase.remaining_stock,
            purchased_at: purchase.purchased_at,
        }),
    ))
}

pub async fn watch_purchase_notifications(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let receiver = state.purchase_events.subscribe();
    let stream = BroadcastStream::new(receiver).filter_map(|result| match result {
        Ok(notification) => match Event::default()
            .event("purchase.completed")
            .json_data(notification)
        {
            Ok(event) => Some(Ok(event)),
            Err(error) => {
                error!(%error, "failed to serialize SSE purchase event");
                None
            }
        },
        Err(error) => {
            warn!(%error, "SSE purchase subscriber lagged");
            None
        }
    });

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_product_id_maps_to_json_bad_request() {
        let (status, Json(response)) = parse_product_id("not-a-number").unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(response.message, "product_id must be a valid integer");
    }

    #[test]
    fn purchase_statuses_map_to_public_http_statuses() {
        let cases = [
            (tonic::Code::InvalidArgument, StatusCode::BAD_REQUEST),
            (tonic::Code::NotFound, StatusCode::NOT_FOUND),
            (tonic::Code::FailedPrecondition, StatusCode::CONFLICT),
            (tonic::Code::DeadlineExceeded, StatusCode::GATEWAY_TIMEOUT),
            (tonic::Code::Unavailable, StatusCode::SERVICE_UNAVAILABLE),
        ];

        for (grpc_code, expected_http_status) in cases {
            let grpc_status = tonic::Status::new(grpc_code, "test");
            let (http_status, Json(_)) = map_inventory_error(grpc_status);
            assert_eq!(http_status, expected_http_status);
        }
    }
}
