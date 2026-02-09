-- PostgreSQL test data initialization
-- This script runs automatically when the container starts

-- Create test schema
CREATE SCHEMA IF NOT EXISTS test_schema;

-- Create sample tables
CREATE TABLE public.users (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    age INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT true
);

CREATE TABLE public.orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER REFERENCES public.users(id),
    product_name VARCHAR(255) NOT NULL,
    quantity INTEGER NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    order_date DATE DEFAULT CURRENT_DATE,
    status VARCHAR(50) DEFAULT 'pending'
);

CREATE TABLE public.products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category VARCHAR(100),
    price DECIMAL(10, 2) NOT NULL,
    stock INTEGER DEFAULT 0,
    description TEXT
);

CREATE TABLE test_schema.analytics (
    id SERIAL PRIMARY KEY,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB,
    timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Insert test data
INSERT INTO public.users (name, email, age, is_active) VALUES
    ('Alice Johnson', 'alice@example.com', 28, true),
    ('Bob Smith', 'bob@example.com', 35, true),
    ('Carol White', 'carol@example.com', 42, false),
    ('David Brown', 'david@example.com', 31, true),
    ('Eve Davis', 'eve@example.com', 25, true);

INSERT INTO public.products (name, category, price, stock, description) VALUES
    ('Laptop Pro', 'Electronics', 1299.99, 50, 'High-performance laptop'),
    ('Wireless Mouse', 'Electronics', 29.99, 200, 'Ergonomic wireless mouse'),
    ('Office Chair', 'Furniture', 249.99, 30, 'Comfortable office chair'),
    ('Desk Lamp', 'Furniture', 49.99, 100, 'LED desk lamp'),
    ('Coffee Mug', 'Kitchen', 12.99, 500, 'Ceramic coffee mug');

INSERT INTO public.orders (user_id, product_name, quantity, price, status) VALUES
    (1, 'Laptop Pro', 1, 1299.99, 'completed'),
    (1, 'Wireless Mouse', 2, 59.98, 'completed'),
    (2, 'Office Chair', 1, 249.99, 'shipped'),
    (3, 'Desk Lamp', 3, 149.97, 'pending'),
    (4, 'Coffee Mug', 5, 64.95, 'completed'),
    (5, 'Laptop Pro', 1, 1299.99, 'processing');

INSERT INTO test_schema.analytics (event_type, event_data) VALUES
    ('page_view', '{"page": "/home", "duration": 120}'),
    ('click', '{"element": "buy_button", "product_id": 1}'),
    ('page_view', '{"page": "/products", "duration": 45}'),
    ('purchase', '{"order_id": 1, "total": 1299.99}'),
    ('page_view', '{"page": "/checkout", "duration": 180}');

-- Create a read-only user for testing readonly mode
CREATE USER flowlike_readonly WITH PASSWORD 'readonly_test';
GRANT CONNECT ON DATABASE flowlike_test TO flowlike_readonly;
GRANT USAGE ON SCHEMA public TO flowlike_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA public TO flowlike_readonly;
GRANT USAGE ON SCHEMA test_schema TO flowlike_readonly;
GRANT SELECT ON ALL TABLES IN SCHEMA test_schema TO flowlike_readonly;

-- Create view for testing
CREATE VIEW public.user_orders AS
SELECT
    u.id as user_id,
    u.name as user_name,
    u.email,
    COUNT(o.id) as order_count,
    COALESCE(SUM(o.price), 0) as total_spent
FROM public.users u
LEFT JOIN public.orders o ON u.id = o.user_id
GROUP BY u.id, u.name, u.email;

ANALYZE;
