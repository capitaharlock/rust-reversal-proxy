use actix_web::{web, HttpRequest, HttpResponse, Error as ActixError};
use reqwest::Client;
use log::{error, debug};
use std::sync::Arc;
use std::error::Error as StdError;
use crate::config::Config;

pub async fn forward_request(
    req: HttpRequest,
    body: web::Bytes,
    client: web::Data<Arc<Client>>,
    config: web::Data<Arc<Config>>
) -> Result<HttpResponse, ActixError> {
    let path = req.uri().path().trim_start_matches("/api/v1");
    let connection_info = req.connection_info().clone();
    let scheme = connection_info.scheme();

    let target_url = match scheme {
        "http" => &config.target_http_url,
        "https" => &config.target_https_url,
        _ => return Ok(HttpResponse::BadRequest().body("Unsupported scheme")),
    };

    let new_url = format!("{}{}", target_url, path);
    
    debug!("Forwarding to URL: {}", new_url);
    debug!("Original request method: {}", req.method());

    // Convert actix_web::http::Method to reqwest::Method
    let method = match reqwest::Method::from_bytes(req.method().as_str().as_bytes()) {
        Ok(m) => m,
        Err(_) => return Ok(HttpResponse::MethodNotAllowed().finish()),
    };

    let mut forwarded_req = client.request(method, &new_url);

    // Forward headers
    for (name, value) in req.headers() {
        if name != "host" {  // Don't forward the Host header
            forwarded_req = forwarded_req.header(name.as_str(), value.as_bytes());
            debug!("Forwarding header: {}={:?}", name, value);
        }
    }

    // Send the request
    match forwarded_req.body(body).send().await {
        Ok(response) => {
            // Convert reqwest::StatusCode to actix_web::http::StatusCode
            let status = actix_web::http::StatusCode::from_u16(response.status().as_u16())
                .unwrap_or(actix_web::http::StatusCode::INTERNAL_SERVER_ERROR);
            
            let mut client_resp = HttpResponse::build(status);
            
            // Forward response headers
            for (name, value) in response.headers() {
                client_resp.append_header((name.as_str(), value.as_bytes()));
                debug!("Forwarding response header: {}={:?}", name, value);
            }

            // Forward response body
            match response.bytes().await {
                Ok(body) => {
                    debug!("Forwarding response body of size: {} bytes", body.len());
                    Ok(client_resp.body(body))
                },
                Err(e) => {
                    error!("Failed to read response body: {:?}", e);
                    Ok(HttpResponse::InternalServerError().finish())
                }
            }
        },
        Err(e) => {
            error!("Failed to forward request: {:?}", e);
            if let Some(url) = e.url() {
                error!("Failed URL: {}", url);
            }
            if let Some(source) = e.source() {
                error!("Error source: {:?}", source);
            }
            Ok(HttpResponse::BadGateway().body(format!("Failed to forward request: {:?}", e)))
        }
    }
}