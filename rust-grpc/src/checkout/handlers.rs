use super::errors::ErrorResponse;
use crate::inventory_v1::GetProductRequest;
use crate::inventory_v1::inventory_client::InventoryClient;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::Serialize;
use std::time::Duration;
use tonic::Request;
use tonic::transport::Channel;

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
        tonic::Code::NotFound => api_error(StatusCode::NOT_FOUND, "product not found"),
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
}

#[derive(Serialize)]
pub struct ProductResponse {
    pub product_id: i64,
    pub name: String,
    pub stock: i64,
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

    let response = ProductResponse {
        product_id: product.product_id,
        name: product.name,
        stock: product.stock,
    };

    Ok((StatusCode::OK, Json(response)))
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
    fn not_found_maps_to_json_not_found() {
        let grpc_status = tonic::Status::not_found("missing");
        let (status, Json(response)) = map_inventory_error(grpc_status);

        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(response.message, "product not found");
    }

    #[test]
    fn deadline_maps_to_json_gateway_timeout() {
        let grpc_status = tonic::Status::deadline_exceeded("slow");
        let (status, Json(response)) = map_inventory_error(grpc_status);

        assert_eq!(status, StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(response.message, "inventory request timed out");
    }

    #[test]
    fn unavailable_maps_to_json_service_unavailable() {
        let grpc_status = tonic::Status::unavailable("offline");
        let (status, Json(response)) = map_inventory_error(grpc_status);

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(response.message, "inventory service is unavailable");
    }
}
