ALTER TABLE logs RENAME TO logs_old;

CREATE TABLE logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    ts_nanos INTEGER NOT NULL,
    level TEXT NOT NULL,
    target TEXT NOT NULL,
    feedback_log_body TEXT,
    module_path TEXT,
    file TEXT,
    line INTEGER,
    chat_id TEXT,
    process_uuid TEXT,
    estimated_bytes INTEGER NOT NULL DEFAULT 0
);

INSERT INTO logs (
    id,
    ts,
    ts_nanos,
    level,
    target,
    feedback_log_body,
    module_path,
    file,
    line,
    chat_id,
    process_uuid,
    estimated_bytes
)
SELECT
    id,
    ts,
    ts_nanos,
    level,
    target,
    message,
    module_path,
    file,
    line,
    chat_id,
    process_uuid,
    estimated_bytes
FROM logs_old;

DROP TABLE logs_old;

CREATE INDEX idx_logs_ts ON logs(ts DESC, ts_nanos DESC, id DESC);
CREATE INDEX idx_logs_chat_id ON logs(chat_id);
CREATE INDEX idx_logs_chat_id_ts ON logs(chat_id, ts DESC, ts_nanos DESC, id DESC);
CREATE INDEX idx_logs_process_uuid_threadless_ts ON logs(process_uuid, ts DESC, ts_nanos DESC, id DESC)
WHERE chat_id IS NULL;
