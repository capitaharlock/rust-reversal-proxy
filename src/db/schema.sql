CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS features (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);

CREATE TABLE IF NOT EXISTS product_features (
    product_id INTEGER REFERENCES products(id),
    feature_id INTEGER REFERENCES features(id),
    period_duration INTERVAL NOT NULL,
    max_requests INTEGER NOT NULL,
    PRIMARY KEY (product_id, feature_id)
);

CREATE TABLE IF NOT EXISTS api_keys (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES users(id),
    product_id INTEGER REFERENCES products(id),
    key VARCHAR(255) UNIQUE NOT NULL
);

CREATE TABLE IF NOT EXISTS periods (
    id SERIAL PRIMARY KEY,
    product_id INTEGER REFERENCES products(id),
    date_start TIMESTAMP NOT NULL,
    date_end TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS usage (
    api_key_id INTEGER REFERENCES api_keys(id),
    period_id INTEGER REFERENCES periods(id),
    request_count INTEGER NOT NULL,
    PRIMARY KEY (api_key_id, period_id)
);