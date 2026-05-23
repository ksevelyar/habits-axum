CREATE TABLE metrics (
  id BIGSERIAL PRIMARY KEY,

  chain_id BIGINT NOT NULL
    REFERENCES chains(id)
    ON DELETE CASCADE,

  date DATE NOT NULL,

  value_integer BIGINT,
  value_float DOUBLE PRECISION,
  value_bool BOOLEAN,

  inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

  CONSTRAINT metrics_single_value_check CHECK (
    num_nonnulls(
      value_integer,
      value_float,
      value_bool
    ) = 1
  ),

  CONSTRAINT metrics_chain_date_unique UNIQUE (chain_id, date)
);
