ALTER TABLE stage1_outputs RENAME COLUMN thread_id TO chat_id;

DROP INDEX idx_stage1_outputs_source_updated_at;
CREATE INDEX idx_stage1_outputs_source_updated_at
    ON stage1_outputs(source_updated_at DESC, chat_id DESC);
