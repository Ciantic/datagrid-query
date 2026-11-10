# Datagrid query builder

- query - select statements
- insert - inserting new rows
- upsert - inserting new rows, without patch semantics, and version columns
- merge - inserting or updating rows, with patch semantics, and version columns
- update - update rows with patch semantics, and version columns
- delete - delete rows by primary keys and version columns

## Experimental

This is experimental, I'm trying to extract this logic out of my other
experiments. I keep seeing that I need this exact logic in multiple languages at
the moment.

