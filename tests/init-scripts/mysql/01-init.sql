-- MySQL test data initialization
-- This script runs automatically when the container starts

-- Use the test database
USE flowlike_test;

-- Create sample tables
CREATE TABLE users (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    age INT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT true
) ENGINE=InnoDB;

CREATE TABLE orders (
    id INT AUTO_INCREMENT PRIMARY KEY,
    user_id INT,
    product_name VARCHAR(255) NOT NULL,
    quantity INT NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    order_date DATE DEFAULT (CURRENT_DATE),
    status VARCHAR(50) DEFAULT 'pending',
    FOREIGN KEY (user_id) REFERENCES users(id)
) ENGINE=InnoDB;

CREATE TABLE products (
    id INT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    category VARCHAR(100),
    price DECIMAL(10, 2) NOT NULL,
    stock INT DEFAULT 0,
    description TEXT
) ENGINE=InnoDB;

-- Insert test data
INSERT INTO users (name, email, age, is_active) VALUES
    ('Alice Johnson', 'alice@example.com', 28, true),
    ('Bob Smith', 'bob@example.com', 35, true),
    ('Carol White', 'carol@example.com', 42, false),
    ('David Brown', 'david@example.com', 31, true),
    ('Eve Davis', 'eve@example.com', 25, true);

INSERT INTO products (name, category, price, stock, description) VALUES
    ('Laptop Pro', 'Electronics', 1299.99, 50, 'High-performance laptop'),
    ('Wireless Mouse', 'Electronics', 29.99, 200, 'Ergonomic wireless mouse'),
    ('Office Chair', 'Furniture', 249.99, 30, 'Comfortable office chair'),
    ('Desk Lamp', 'Furniture', 49.99, 100, 'LED desk lamp'),
    ('Coffee Mug', 'Kitchen', 12.99, 500, 'Ceramic coffee mug');

INSERT INTO orders (user_id, product_name, quantity, price, status) VALUES
    (1, 'Laptop Pro', 1, 1299.99, 'completed'),
    (1, 'Wireless Mouse', 2, 59.98, 'completed'),
    (2, 'Office Chair', 1, 249.99, 'shipped'),
    (3, 'Desk Lamp', 3, 149.97, 'pending'),
    (4, 'Coffee Mug', 5, 64.95, 'completed'),
    (5, 'Laptop Pro', 1, 1299.99, 'processing');

-- Create a read-only user for testing readonly mode
CREATE USER IF NOT EXISTS 'flowlike_readonly'@'%' IDENTIFIED BY 'readonly_test';
GRANT SELECT ON flowlike_test.* TO 'flowlike_readonly'@'%';
FLUSH PRIVILEGES;

-- Create view for testing
CREATE VIEW user_orders AS
SELECT
    u.id as user_id,
    u.name as user_name,
    u.email,
    COUNT(o.id) as order_count,
    COALESCE(SUM(o.price), 0) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name, u.email;
