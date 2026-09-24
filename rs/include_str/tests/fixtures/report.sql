-- Comments and whitespace in a realistic CTE query.
WITH totals AS (
    SELECT account_id, SUM(amount) AS total
    FROM payments /* only settled payments */
    WHERE status = 'settled' AND note <> 'not -- a comment'
    GROUP BY account_id
)
SELECT a.id, 'O''Brien /* literal */' AS label, t.total
FROM accounts AS a
JOIN totals AS t ON t.account_id = a.id
WHERE t.total >= $1 AND a.name <> '  keep these spaces  '
ORDER BY t.total DESC;
