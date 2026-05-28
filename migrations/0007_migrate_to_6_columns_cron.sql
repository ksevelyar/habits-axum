UPDATE tasks
SET cron = '0 ' || cron
WHERE array_length(string_to_array(cron, ' '), 1) = 5;
