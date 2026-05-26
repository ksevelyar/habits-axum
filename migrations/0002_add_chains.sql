DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'chain_type') THEN
        CREATE TYPE chain_type AS ENUM ('integer', 'float', 'boolean');
    END IF;
END$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'chain_aggregate') THEN
        CREATE TYPE chain_aggregate AS ENUM ('sum', 'avg');
    END IF;
END$$;

CREATE TABLE IF NOT EXISTS chains (
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

CREATE INDEX IF NOT EXISTS idx_chains_user_id ON chains(user_id);
