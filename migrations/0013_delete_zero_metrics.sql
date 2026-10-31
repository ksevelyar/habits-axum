DELETE FROM metrics
WHERE value_integer = 0
   OR value_float = 0
   OR value_bool = FALSE;
