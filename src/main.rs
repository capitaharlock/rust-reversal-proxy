use actix_web::{web, App, HttpServer, HttpRequest};
use log::{debug, error};
use num_cpus;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;

use reverse_proxy::{
    handlers, 
    config::Config, 
    db, 
    middleware::Middleware
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = Arc::new(Config::from_env().map_err(|e| {
        error!("Failed to load configuration: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?);

    let pg_client = db::connect_to_postgres(&config.database_url).await.map_err(|e| {
        error!("Failed to connect to database: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    db::init_db(&pg_client).await.map_err(|e| {
        error!("Failed to initialize database: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    let api_keys: HashMap<String, bool> = db::load_api_keys(&pg_client).await.map_err(|e| {
        error!("Failed to load API keys: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e)
    })?;

    let client = Arc::new(Client::new());
    let config_clone = config.clone();

    let middleware = Middleware::new(
        api_keys,
        &config.redis_url,
        config.http_requests_per_minute,
        config.ws_connections_per_minute,
    ).map_err(|e| {
        error!("Failed to create middleware: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, "Middleware creation failed")
    })?;

    HttpServer::new(move || {
        let config = config_clone.clone();
        App::new()
            .wrap(middleware.clone()) 
            .app_data(web::Data::new(client.clone()))
            .app_data(web::Data::new(config.clone()))
            .service(
                web::scope("/ws").route("/{tail:.*}", web::get().to(
                    |req: HttpRequest, payload: web::Payload, config: web::Data<Arc<Config>>| async move {
                        debug!("WebSocket route matched for path: {}", req.path());
                        handlers::ws::ws_handler(req, payload, config).await
                    }
                ))
            )
            .default_service(
                web::to(
                    |req: HttpRequest, body: web::Bytes, client: web::Data<Arc<Client>>, config: web::Data<Arc<Config>>| async move {
                        debug!("Default route matched for path: {}", req.path());
                        handlers::regular::forward_request(req, body, client, config).await
                    }
                ))
    })
    .workers(num_cpus::get())
    .max_connections(1000)
    .bind(("0.0.0.0", config.port))?
    .run()
    .await
}