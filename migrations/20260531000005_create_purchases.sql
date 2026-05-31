CREATE TABLE IF NOT EXISTS purchases (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    product_id UUID NOT NULL REFERENCES products(id) ON DELETE CASCADE,
    payment_gateway VARCHAR(50) NOT NULL,
    gateway_payment_id VARCHAR(255) NOT NULL,
    customer_email VARCHAR(255) NOT NULL,
    fulfilled BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_purchases_gateway_payment_id ON purchases(gateway_payment_id);
