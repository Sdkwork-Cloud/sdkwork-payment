ALTER TABLE commerce_payment_certificate ADD COLUMN IF NOT EXISTS ciphertext TEXT;
ALTER TABLE commerce_payment_certificate ADD COLUMN IF NOT EXISTS encryption_key_id TEXT;
ALTER TABLE commerce_payment_certificate ADD COLUMN IF NOT EXISTS encryption_algorithm TEXT;
