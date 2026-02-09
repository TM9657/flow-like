-- ClickHouse test data initialization
-- This script runs automatically when the container starts

-- Create database
CREATE DATABASE IF NOT EXISTS flowlike_test;

-- Create users table
CREATE TABLE flowlike_test.users (
    id UInt32,
    name String,
    email String,
    age UInt8,
    created_at DateTime DEFAULT now(),
    is_active UInt8 DEFAULT 1
) ENGINE = MergeTree()
ORDER BY id;

-- Create orders table
CREATE TABLE flowlike_test.orders (
    id UInt32,
    user_id UInt32,
    product_name String,
    quantity UInt32,
    price Decimal(10, 2),
    order_date Date DEFAULT today(),
    status String DEFAULT 'pending'
) ENGINE = MergeTree()
ORDER BY (id, order_date);

-- Create products table
CREATE TABLE flowlike_test.products (
    id UInt32,
    name String,
    category String,
    price Decimal(10, 2),
    stock UInt32 DEFAULT 0,
    description String
) ENGINE = MergeTree()
ORDER BY id;

-- Create analytics table (for time-series testing)
CREATE TABLE flowlike_test.analytics (
    id UInt64,
    event_type String,
    event_data String,
    timestamp DateTime DEFAULT now()
) ENGINE = MergeTree()
ORDER BY (timestamp, id);

-- Insert test data
INSERT INTO flowlike_test.users (id, name, email, age, is_active) VALUES
    (1, 'Alice Johnson', 'alice@example.com', 28, 1),
    (2, 'Bob Smith', 'bob@example.com', 35, 1),
    (3, 'Carol White', 'carol@example.com', 42, 0),
    (4, 'David Brown', 'david@example.com', 31, 1),
    (5, 'Eve Davis', 'eve@example.com', 25, 1);

INSERT INTO flowlike_test.products (id, name, category, price, stock, description) VALUES
    (1, 'Laptop Pro', 'Electronics', 1299.99, 50, 'High-performance laptop'),
    (2, 'Wireless Mouse', 'Electronics', 29.99, 200, 'Ergonomic wireless mouse'),
    (3, 'Office Chair', 'Furniture', 249.99, 30, 'Comfortable office chair'),
    (4, 'Desk Lamp', 'Furniture', 49.99, 100, 'LED desk lamp'),
    (5, 'Coffee Mug', 'Kitchen', 12.99, 500, 'Ceramic coffee mug');

INSERT INTO flowlike_test.orders (id, user_id, product_name, quantity, price, status) VALUES
    (1, 1, 'Laptop Pro', 1, 1299.99, 'completed'),
    (2, 1, 'Wireless Mouse', 2, 59.98, 'completed'),
    (3, 2, 'Office Chair', 1, 249.99, 'shipped'),
    (4, 3, 'Desk Lamp', 3, 149.97, 'pending'),
    (5, 4, 'Coffee Mug', 5, 64.95, 'completed'),
    (6, 5, 'Laptop Pro', 1, 1299.99, 'processing');

INSERT INTO flowlike_test.analytics (id, event_type, event_data) VALUES
    (1, 'page_view', '{"page": "/home", "duration": 120}'),
    (2, 'click', '{"element": "buy_button", "product_id": 1}'),
    (3, 'page_view', '{"page": "/products", "duration": 45}'),
    (4, 'purchase', '{"order_id": 1, "total": 1299.99}'),
    (5, 'page_view', '{"page": "/checkout", "duration": 180}');

-- Create a read-only user
CREATE USER IF NOT EXISTS flowlike_readonly IDENTIFIED BY 'readonly_test';
GRANT SELECT ON flowlike_test.* TO flowlike_readonly;
