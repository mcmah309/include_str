/* Body whitespace and comments must be preserved exactly. */
CREATE FUNCTION example() RETURNS text AS $body$
BEGIN
    -- This comment belongs to the function body.
    /* So does this one. */
    RETURN 'a  b -- literal';
END;
$body$ LANGUAGE plpgsql;
-- Outside the body, comments can be removed.
SELECT E'it\'s  -- still a literal';
