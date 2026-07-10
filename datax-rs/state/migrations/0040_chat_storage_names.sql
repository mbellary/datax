ALTER TABLE thread_dynamic_tools RENAME TO chat_dynamic_tools;
ALTER TABLE chat_dynamic_tools RENAME COLUMN thread_id TO chat_id;
DROP INDEX IF EXISTS idx_thread_dynamic_tools_thread;
CREATE INDEX idx_chat_dynamic_tools_chat ON chat_dynamic_tools(chat_id);

ALTER TABLE thread_spawn_edges RENAME TO chat_spawn_edges;
ALTER TABLE chat_spawn_edges RENAME COLUMN parent_thread_id TO parent_chat_id;
ALTER TABLE chat_spawn_edges RENAME COLUMN child_thread_id TO child_chat_id;
DROP INDEX IF EXISTS idx_thread_spawn_edges_parent_status;
CREATE INDEX idx_chat_spawn_edges_parent_status
    ON chat_spawn_edges(parent_chat_id, status);

ALTER TABLE agent_job_items RENAME COLUMN assigned_thread_id TO assigned_chat_id;
