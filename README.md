# Rust Reverse Proxy

This project provides a **reverse proxy service** that validates API keys and forwards requests to the target API. The service is designed with minimal latency in mind, built in **Rust**, and supports both HTTP and WebSocket requests.  

---

## Features  
- Validates API keys from the header: `X-Api-Key: {api_key}`.  
- Proxies both HTTP and WebSocket requests.  
- Handles HTTP upgrades for WebSocket connections.  
- Stores API key information securely in **PostgreSQL**.  
- Packaged as a **Docker image**, with configuration provided via environment variables.  

---

## Specifications  

### **Docker Restart Commands**  
To restart the service, run:  
```bash
docker-compose down -v
docker-compose up --build
