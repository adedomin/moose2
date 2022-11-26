pub const CREATE_TABLE: &str = r###"
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS Moose (
    name       TEXT    PRIMARY KEY,
    pos        INTEGER NOT NULL,
    image      BLOB    NOT NULL,
    dimensions INTEGER NOT NULL,
    created    TEXT    NOT NULL,
    author     TEXT    DEFAULT NULL,
    deleted    INTEGER DEFAULT 0
) WITHOUT ROWID;
CREATE UNIQUE INDEX IF NOT EXISTS Moose_NameIdx ON Moose (name);
CREATE INDEX IF NOT EXISTS Moose_AuthorIdx ON Moose (author);
-- They are not unique to make renumbering them easier.
CREATE INDEX IF NOT EXISTS Moose_PosIdx ON Moose (pos);

CREATE VIRTUAL TABLE IF NOT EXISTS MooseSearch USING fts5(
    moose_name, tokenize = 'porter unicode61'
);

CREATE TRIGGER IF NOT EXISTS Moose_InsertTrigger AFTER INSERT ON Moose
BEGIN
    INSERT INTO MooseSearch(moose_name) VALUES (NEW.name);
END;

CREATE TRIGGER IF NOT EXISTS Moose_DeleteTrigger AFTER DELETE ON Moose
BEGIN
    DELETE FROM MooseSearch WHERE moose_name = OLD.name;
    -- Deletes happen through sqlite3 shell, not the app.
    UPDATE Moose SET pos = pos - 1 WHERE pos > OLD.pos;
END;
"###;

pub const INSERT_MOOSE: &str =
    "INSERT INTO Moose(name, pos, image, dimensions, created, author) VALUES (?, ?, ?, ?, ?, ?)";

pub const LAST_MOOSE: &str = r###"
    SELECT name, image, dimensions, created, author
    FROM Moose
    WHERE pos = ( SELECT MAX(pos) FROM Moose )
"###;

pub const LEN_MOOSE: &str = "SELECT MAX(pos) FROM Moose";

pub const GET_MOOSE: &str =
    "SELECT name, image, dimensions, created, author FROM Moose WHERE name = ?";

pub const GET_MOOSE_IDX: &str =
    "SELECT name, image, dimensions, created, author FROM Moose WHERE pos = ?";

pub const GET_MOOSE_PAGE: &str = r###"
    SELECT name, image, dimensions, created, author
    FROM Moose
    WHERE pos >= ? AND pos < ?
    ORDER BY pos
"###;

pub const SEARCH_MOOSE_PAGE: &str = r###"
    SELECT pos, name, image, dimensions, created, author
    FROM Moose
    INNER JOIN (
        SELECT moose_name FROM MooseSearch
        WHERE moose_name MATCH ?
        ORDER BY RANK
        LIMIT 50
    )
    ON name == moose_name
"###;

pub const INSERT_MOOSE_WITH_COMPUTED_POS: &str = r###"
    INSERT INTO Moose(name,                              pos, image, dimensions, created, author)
    VALUES           (   ?, (SELECT MAX(pos) FROM Moose) + 1,     ?,          ?,       ?,      ?);
"###;
