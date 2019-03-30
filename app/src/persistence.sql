CREATE TABLE IF NOT EXISTS documents (
    id TEXT,
    body jsonb NOT NULL,
    PRIMARY KEY(id)
);
UPDATE documents
    SET body = jsonb_set(body, '{_id}', to_jsonb(id))
    WHERE coalesce(id != (body ->> '_id') , true);
ALTER TABLE documents ADD CHECK (id = (body ->> '_id'));