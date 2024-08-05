DROP TABLE IF EXISTS tasks;
DROP SEQUENCE total_tasks;

DROP FUNCTION IF EXISTS check_task_data;
DROP FUNCTION IF EXISTS get_task_priority_level;
DROP FUNCTION IF EXISTS get_worker_id_from_task;

DROP TYPE IF EXISTS task_status;
DROP TYPE IF EXISTS task_priority;
