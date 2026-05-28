UPDATE tasks
SET user_id = (
    SELECT id
    FROM users
    WHERE email = 'ksevelyar@gmail.com'
)
WHERE user_id IS NULL;
