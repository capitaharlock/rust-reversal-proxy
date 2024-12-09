use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use std::future::{ready, Ready};
use std::pin::Pin;
use std::sync::Arc;
use std::collections::HashMap;
use actix_web::error::{ErrorTooManyRequests, ErrorUnauthorized};
use redis::{Client, Commands, RedisResult};

pub struct RateLimiter {
    client: Client,
    limit: u32,
    window: i64,
    prefix: String,
}

impl RateLimiter {
    fn new(redis_url: &str, limit: u32, window: u64, prefix: &str) -> RedisResult<Self> {
        let client = Client::open(redis_url)?;
        Ok(RateLimiter {
            client,
            limit,
            window: window as i64,
            prefix: prefix.to_string(),
        })
    }

    fn check(&self, ip: &str) -> Result<(), Error> {
        let mut con = self.client.get_connection()
            .map_err(|e| Error::from(ErrorTooManyRequests(format!("Redis error: {}", e))))?;
        let key = format!("{}:rate_limit:{}", self.prefix, ip);

        let current_count: u32 = con.get(&key).unwrap_or(0);

        if current_count >= self.limit {
            return Err(ErrorTooManyRequests("Rate limit exceeded"));
        }

        let count: u32 = con.incr(&key, 1)
            .map_err(|e| Error::from(ErrorTooManyRequests(format!("Redis error: {}", e))))?;

        if count == 1 {
            con.expire(&key, self.window)
                .map_err(|e| Error::from(ErrorTooManyRequests(format!("Redis error: {}", e))))?;
        }

        Ok(())
    }
}

pub struct Middleware {
    api_keys: Arc<HashMap<String, bool>>,
    http_limiter: Arc<RateLimiter>,
    ws_limiter: Arc<RateLimiter>,
}

impl Middleware {
    pub fn new(api_keys: HashMap<String, bool>, redis_url: &str, http_limit: u32, ws_limit: u32) -> RedisResult<Self> {
        Ok(Middleware {
            api_keys: Arc::new(api_keys),
            http_limiter: Arc::new(RateLimiter::new(redis_url, http_limit, 60, "http")?),
            ws_limiter: Arc::new(RateLimiter::new(redis_url, ws_limit, 60, "ws")?),
        })
    }

    fn check_api_key(&self, req: &ServiceRequest) -> Result<(), Error> {
        match req.headers().get("x-api-key").and_then(|h| h.to_str().ok()) {
            Some(key) if self.api_keys.contains_key(key) => Ok(()),
            Some(_) => Err(ErrorUnauthorized("Invalid API Key")),
            None => Err(ErrorUnauthorized("Missing API Key")),
        }
    }

    fn check_rate_limit(&self, req: &ServiceRequest) -> Result<(), Error> {
        let ip = req.connection_info().realip_remote_addr()
            .unwrap_or("unknown")
            .to_string();
        let is_websocket = req.headers().contains_key("Sec-WebSocket-Key") || req.path().starts_with("/ws");

        if is_websocket {
            self.ws_limiter.check(&ip)
        } else {
            self.http_limiter.check(&ip)
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for Middleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = MiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MiddlewareService {
            service,
            inner: self.clone(),
        }))
    }
}

pub struct MiddlewareService<S> {
    service: S,
    inner: Middleware,
}

impl<S, B> Service<ServiceRequest> for MiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>>>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let inner = self.inner.clone();

        if let Err(e) = inner.check_api_key(&req) {
            return Box::pin(async { Err(e) });
        }

        if let Err(e) = inner.check_rate_limit(&req) {
            return Box::pin(async { Err(e) });
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}

impl Clone for Middleware {
    fn clone(&self) -> Self {
        Middleware {
            api_keys: Arc::clone(&self.api_keys),
            http_limiter: Arc::clone(&self.http_limiter),
            ws_limiter: Arc::clone(&self.ws_limiter),
        }
    }
}