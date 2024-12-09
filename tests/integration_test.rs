use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::StatusCode;
use serde_json::{json, Value};

const BASE_URL: &str = "http://127.0.0.1:8080/api/v1/api";
const API_KEY: &str = "1a79a4d60de6718e1a79a4d60de6718e";

fn create_client() -> Client {
    Client::new()
}

fn create_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert("x-api-key", HeaderValue::from_static(API_KEY));
    headers
}

fn send_request(client: &Client, method: reqwest::Method, path: &str, body: Option<Value>) -> reqwest::Result<(StatusCode, String)> {
    let url = format!("{}{}", BASE_URL, path);
    let mut request = client.request(method, &url).headers(create_headers());
    
    if let Some(json_body) = body {
        request = request.json(&json_body);
    }

    let response = request.send()?;
    let status = response.status();
    let body = response.text()?;

    Ok((status, body))
}

#[test]
fn test_get_market_data() {
    let client = create_client();
    let (status, body) = send_request(&client, reqwest::Method::GET, "/market-data?symbol=BTC/USD", None).expect("Failed to send request");

    assert!(status.is_success(), "GET market data request failed with status: {}", status);
    assert!(!body.is_empty(), "Response body should not be empty");
}

#[test]
fn test_place_order() {
    let client = create_client();
    let new_order = json!({
        "symbol": "ETH/USD",
        "side": "buy",
        "amount": 1.5
    });

    let (status, body) = send_request(&client, reqwest::Method::POST, "/orders", Some(new_order)).expect("Failed to send request");

    assert_eq!(status, StatusCode::CREATED, "POST order request failed with status: {}", status);
    assert!(!body.is_empty(), "Response body should not be empty");
}

#[test]
fn test_get_orders() {
    let client = create_client();
    let (status, body) = send_request(&client, reqwest::Method::GET, "/orders", None).expect("Failed to send request");

    assert!(status.is_success(), "GET orders request failed with status: {}", status);
    assert!(!body.is_empty(), "Response body should not be empty");
}

#[test]
fn test_cancel_order() {
    let client = create_client();
    
    // Place an order
    let new_order = json!({
        "symbol": "XRP/USD",
        "side": "sell",
        "amount": 1000
    });
    let (place_status, place_body) = send_request(&client, reqwest::Method::POST, "/orders", Some(new_order)).expect("Failed to place order");
    assert_eq!(place_status, StatusCode::CREATED, "Failed to place order for cancellation test");

    let order_id = serde_json::from_str::<Value>(&place_body)
        .expect("Failed to parse order response")
        ["id"].as_str()
        .expect("Failed to extract order ID")
        .to_string();

    // Cancel the order
    let cancel_path = format!("/orders/{}", order_id);
    let (cancel_status, _) = send_request(&client, reqwest::Method::DELETE, &cancel_path, None).expect("Failed to cancel order");

    assert!(cancel_status.is_success(), "DELETE order request failed with status: {}", cancel_status);
}

#[test]
fn test_get_balance() {
    let client = create_client();
    let (status, body) = send_request(&client, reqwest::Method::GET, "/balance", None).expect("Failed to send request");

    assert!(status.is_success(), "GET balance request failed with status: {}", status);
    assert!(!body.is_empty(), "Response body should not be empty");
}