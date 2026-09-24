-- Different identifier quote styles, escaped delimiters, and JSON.
SELECT "a""b -- c", `a``b /* c */`, [a]]b  c],
       '{"message": "two  spaces -- /* literal */", "path": "C:\\tmp"}' AS payload,
       '日本語 🦀
  keep this newline' AS unicode_text
FROM "odd  table";
