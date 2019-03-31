-- The implementation will split this file on a pair of newlines, each
-- statement needs to be over consecutive lines.

CREATE TABLE IF NOT EXISTS _migrations (
    id TEXT PRIMARY KEY,
    md5_digest TEXT
);

DROP FUNCTION IF EXISTS  _migrate;
DROP PROCEDURE IF EXISTS  _migrate;
CREATE PROCEDURE _migrate(migration_id text, migration_sql text) AS $$
DECLARE
    digest TEXT := md5(migration_sql);
    known_digest TEXT;
BEGIN
    SELECT md5_digest INTO known_digest FROM _migrations
        WHERE _migrations.id = migration_id;
    SELECT md5(migration_sql) INTO digest;
    RAISE NOTICE 'Name: %; Known: %; current:%;', migration_id, known_digest, digest;
    IF known_digest IS NULL THEN
        RAISE NOTICE 'Applying change % with digest %', migration_id, digest;
    ELSIF known_digest != digest THEN
        RAISE EXCEPTION 'Digest for migration % has changed from % to %',
            migration_id, known_digest, digest;
    ELSE
        RAISE NOTICE 'Change % with digest % already applied', migration_id, digest;
        RETURN;
    END IF;
    EXECUTE migration_sql;
    INSERT INTO _migrations (id, md5_digest) VALUES (migration_id, digest);
END;
$$ LANGUAGE 'plpgsql';

CALL _migrate(text '0001 create documents', text $$
    CREATE TABLE IF NOT EXISTS documents (
        id TEXT,
        body jsonb NOT NULL,
        PRIMARY KEY(id)
    );
$$);

CALL _migrate(text '0002 add check for id coherence', text $$
    UPDATE documents
        SET body = jsonb_set(body, '{_id}', to_jsonb(id))
        WHERE coalesce(id != (body ->> '_id') , true);
    ALTER TABLE documents ADD CONSTRAINT id_coherence
        CHECK ((body ->> '_id') IS NOT NULL AND  id = (body ->> '_id'));
$$);

CALL _migrate(text '0003 Ensure all documents have versions', text $$
    UPDATE documents
        SET body = jsonb_set(body, '{_version}', to_jsonb(to_hex(txid_current())))
        WHERE (body ->> '_version') IS NULL;
$$);