
## DEX specs
Authenticated Api Key Requirements
Reverse proxy that receives a request with a header “X-Api-Key”: {api_key} that validates its a valid api before forwarding the request to our api
Reverse proxy service needs to have minimal latency and should be built in either rust or golang
Able to proxy a websocket request 
Handles http upgrade
Should be packaged as a docker image with configuration (postgres url ect) loaded from environment variables
Api key information should be stored within postgres

## Docker restart
docker-compose down -v
docker-compose up --build

## Docker routing !!!
Windows: ENV TARGET_API_URL=http://host.docker.internal:2345
Linux: ENV TARGET_API_URL=http://172.17.0.1:2345

## Test
cargo test -- --nocapture

## Db scheme
api_keys (id, userId, productId, key)
usage (apiKeyId, periodId, request_count)
periods (id, productId, dateStart, dateEnd)
users (id, email, password)
products (id, name)
features (id, name)
product_features (productId, featureId, period_duration, max_requests)