use include_str::include_str;

#[test]
fn compacts_a_complete_cte_without_changing_literals_or_parameters() {
    const SQL: &str = include_str!("fixtures/report.sql" => sql);
    assert_eq!(
        SQL,
        "WITH totals AS ( SELECT account_id, SUM(amount) AS total FROM payments WHERE status = 'settled' AND note <> 'not -- a comment' GROUP BY account_id ) SELECT a.id, 'O''Brien /* literal */' AS label, t.total FROM accounts AS a JOIN totals AS t ON t.account_id = a.id WHERE t.total >= $1 AND a.name <> '  keep these spaces  ' ORDER BY t.total DESC;"
    );
}

#[test]
fn preserves_a_complete_procedure_body_byte_for_byte() {
    const SQL: &str = include_str!("fixtures/procedure.sql" => sql);
    assert_eq!(
        SQL,
        "CREATE FUNCTION example() RETURNS text AS $body$\nBEGIN\n    -- This comment belongs to the function body.\n    /* So does this one. */\n    RETURN 'a  b -- literal';\nEND;\n$body$ LANGUAGE plpgsql; SELECT E'it\\'s  -- still a literal';"
    );
}

#[test]
fn preserves_json_multiline_unicode_and_escaped_identifier_delimiters() {
    const SQL: &str = include_str!("fixtures/quoted.sql" => sql);
    assert_eq!(
        SQL,
        r#"SELECT "a""b -- c", `a``b /* c */`, [a]]b  c], '{"message": "two  spaces -- /* literal */", "path": "C:\\tmp"}' AS payload, '日本語 🦀
  keep this newline' AS unicode_text FROM "odd  table";"#
    );
}
