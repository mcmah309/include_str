-- Fetch active users
SELECT  id,  'not -- a /* comment */' AS label
FROM/* separator */users
/* outer /* nested */ comment */WHERE active = 1; -- end