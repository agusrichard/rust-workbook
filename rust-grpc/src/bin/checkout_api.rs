use rust_grpc::inventory_v1::GetAvailabilityRequest;
use rust_grpc::inventory_v1::inventory_client::InventoryClient;
use std::time::Duration;
use tonic::Request;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<String>>();
    let id = args.get(1).unwrap();
    let id = id.parse::<i64>()?;

    let mut client = InventoryClient::connect("http://[::1]:50051").await?;

    let mut request = Request::new(GetAvailabilityRequest { product_id: id });

    request.set_timeout(Duration::from_secs(2));

    let product = client.get_availability(request).await?.into_inner();
    println!("{}: {} in stock", product.name, product.available_quantity);

    Ok(())
}
