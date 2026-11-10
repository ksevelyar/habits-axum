CREATE TABLE ledger_categories (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT ledger_categories_user_name_unique UNIQUE (user_id, name)
);

CREATE INDEX ledger_categories_user_id_index ON ledger_categories(user_id, id);

CREATE TABLE ledger_transactions (
    id BIGSERIAL PRIMARY KEY,
    category_id BIGINT NOT NULL REFERENCES ledger_categories(id) ON DELETE RESTRICT,
    amount_kopecks BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX ledger_transactions_category_occurred_at_index
    ON ledger_transactions(category_id, occurred_at DESC, id DESC);
