CREATE TYPE chain_type AS ENUM ('integer', 'float', 'boolean');
CREATE TYPE chain_aggregate AS ENUM ('sum', 'avg');

CREATE TABLE chains (
    id BIGSERIAL PRIMARY KEY,

    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    active BOOLEAN NOT NULL DEFAULT TRUE,
    name TEXT NOT NULL,
    type chain_type NOT NULL,
    aggregate chain_aggregate NOT NULL,
    description TEXT,
    "order" INTEGER,

    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_chains_user_id ON chains(user_id);
