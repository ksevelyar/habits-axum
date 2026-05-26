CREATE TABLE tasks (
    id BIGSERIAL PRIMARY KEY,

    name VARCHAR(255) NOT NULL,
    active BOOLEAN NOT NULL DEFAULT FALSE,
    cron VARCHAR(255) NOT NULL,

    user_id BIGINT NOT NULL
      REFERENCES users(id)
      ON DELETE CASCADE,

    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_tasks_user_id ON tasks(user_id);
