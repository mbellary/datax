ALTER TABLE logs RENAME COLUMN thread_id TO chat_id;
DROP INDEX IF EXISTS idx_logs_thread_id;
DROP INDEX IF EXISTS idx_logs_thread_id_ts;
DROP INDEX IF EXISTS idx_logs_process_uuid_threadless_ts;
CREATE INDEX idx_logs_chat_id ON logs(chat_id);
CREATE INDEX idx_logs_chat_id_ts
    ON logs(chat_id, ts DESC, ts_nanos DESC, id DESC);
CREATE INDEX idx_logs_process_uuid_threadless_ts
    ON logs(process_uuid, ts DESC, ts_nanos DESC, id DESC)
    WHERE chat_id IS NULL;
