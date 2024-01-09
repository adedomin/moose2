pub const CREATE_TABLE: &str = r###"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS Moose
  ( name       TEXT    PRIMARY KEY
  -- pos is unique for all intents and purposes, but not made so
  -- to make it easier to renumber. It's used for keyset offsetting.
  , pos        INTEGER NOT NULL
  , image      BLOB    NOT NULL
  , dimensions INTEGER NOT NULL
  , created    TEXT    NOT NULL
  , author     TEXT    DEFAULT NULL
  -- it's either this or N*M joining on Vote table.
  , upvotes    INTEGER DEFAULT 0
  ) WITHOUT ROWID;
CREATE UNIQUE INDEX IF NOT EXISTS Moose_NameIdx   ON Moose(name);
CREATE        INDEX IF NOT EXISTS Moose_AuthorIdx ON Moose(author);
CREATE        INDEX IF NOT EXISTS Moose_PosIdx    ON Moose(pos);

CREATE VIRTUAL TABLE IF NOT EXISTS MooseSearch USING fts5
  ( moose_name, tokenize = 'porter unicode61' );

CREATE TRIGGER IF NOT EXISTS Moose_InsertTrigger
AFTER INSERT ON Moose
BEGIN
  INSERT INTO MooseSearch(moose_name) VALUES (NEW.name);
END;

CREATE TRIGGER IF NOT EXISTS Moose_DeleteTrigger
AFTER DELETE ON Moose
BEGIN
  DELETE FROM MooseSearch WHERE moose_name = OLD.name;
  -- Deletes happen through sqlite3 shell, not the app.
  UPDATE Moose SET pos = pos - 1 WHERE pos > OLD.pos;
END;

CREATE TABLE IF NOT EXISTS Vote
  ( author_name TEXT    NOT NULL
  , moose_name  TEXT    NOT NULL
  , vote_type   INTEGER DEFAULT 0
  , FOREIGN KEY (moose_name) REFERENCES Moose (name) ON DELETE CASCADE
  , PRIMARY KEY (author_name, moose_name)
  ) WITHOUT ROWID;
CREATE        INDEX IF NOT EXISTS Vote_ByMNameIdx on Vote(moose_name);


CREATE TRIGGER IF NOT EXISTS Vote_UpdateTrigger
AFTER INSERT ON Vote
BEGIN
  UPDATE Moose
     SET upvotes = upvotes + NEW.vote_type
   WHERE name = OLD.moose_name;
END;

CREATE TRIGGER IF NOT EXISTS Vote_UpdateTrigger
AFTER UPDATE ON Vote
BEGIN
  UPDATE Moose
     SET upvotes = ( upvotes - OLD.vote_type ) + NEW.vote_type
   WHERE name = OLD.moose_name;
END;

CREATE TRIGGER IF NOT EXISTS Vote_UpdateTrigger
AFTER DELETE ON Vote
BEGIN
  UPDATE Moose
     SET upvotes = upvotes - OLD.vote_type
   WHERE name = OLD.moose_name;
END;
"###;

pub const INSERT_MOOSE: &str =
    "INSERT INTO Moose(name, pos, image, dimensions, created, author, upvotes) VALUES (?, ?, ?, ?, ?, ?, 0)";

pub const INSERT_VOTE: &str =
    "INSERT INTO Vote(author_name, moose_name, vote_type) VALUES (?, ?, ?)";

pub const LAST_MOOSE: &str = r###"
    SELECT name, image, dimensions, created, author, upvotes
      FROM Moose
     WHERE pos = ( SELECT MAX(pos) FROM Moose )
"###;

pub const LEN_MOOSE: &str = "SELECT MAX(pos) FROM Moose";

pub const GET_MOOSE: &str =
    "SELECT name, image, dimensions, created, author, upvotes FROM Moose WHERE name = ?";

pub const GET_MOOSE_IDX: &str =
    "SELECT name, image, dimensions, created, author, upvotes FROM Moose WHERE pos = ?";

pub const GET_MOOSE_PAGE: &str = r###"
    SELECT m.name
         , m.image
         , m.dimensions
         , m.created
         , m.author
         , m.upvotes
      FROM Moose m
     WHERE m.pos >= ? AND m.pos < ?
     ORDER BY pos
"###;

pub const SEARCH_MOOSE_PAGE: &str = const_format::formatcp!(
    r###"
    SELECT m.pos
         , m.name
         , m.image
         , m.dimensions
         , m.created
         , m.author
         , m.upvotes
      FROM Moose m
INNER JOIN
         ( SELECT moose_name
             FROM MooseSearch
            WHERE moose_name MATCH ?
            ORDER BY RANK
            LIMIT {0}
         )
        ON m.name == moose_name
"###,
    crate::model::PAGE_SIZE * crate::model::PAGE_SEARCH_LIM
);

pub const INSERT_MOOSE_WITH_COMPUTED_POS: &str = r###"
    INSERT INTO Moose(name,                              pos, image, dimensions, created, author)
    VALUES           (   ?, (SELECT MAX(pos) FROM Moose) + 1,     ?,          ?,       ?,      ?);
"###;

pub const DUMP_MOOSE: &str = "SELECT name, image, dimensions, created, author, upvotes FROM Moose";
