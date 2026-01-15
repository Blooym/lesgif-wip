CREATE TABLE accounts(
  did TEXT PRIMARY KEY,
  display_name TEXT,
  description TEXT,
  pronouns TEXT,
  avatar_blob_cid TEXT,
  handle TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  status TEXT NOT NULL DEFAULT 'active' 
);

CREATE TABLE labels(
  subject TEXT NOT NULL,
  rkey TEXT NOT NULL,
  value TEXT NOT NULL,
  reason TEXT DEFAULT NULL,
  actor TEXT DEFAULT NULL,
  created_at BIGINT NOT NULL,
  expires_at BIGINT DEFAULT NULL,
  ingested_at BIGINT NOT NULL,
  PRIMARY KEY (subject, rkey)
);

CREATE TABLE posts(
    did TEXT NOT NULL REFERENCES accounts(did) ON DELETE CASCADE,
    rkey TEXT NOT NULL,
    title TEXT NOT NULL,
    tags TEXT[],
    languages TEXT[],
    blob_cid TEXT NOT NULL,
    blob_mime_type TEXT NOT NULL,
    blob_alt_text TEXT,
    created_at BIGINT NOT NULL,
    edited_at BIGINT,
    ingested_at BIGINT NOT NULL,
    PRIMARY KEY(did, rkey)
);

CREATE TABLE post_favourites(
  did TEXT NOT NULL REFERENCES accounts(did) ON DELETE CASCADE,
  rkey TEXT NOT NULL,
  post_did TEXT NOT NULL,
  post_rkey TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  ingested_at BIGINT NOT NULL,
  PRIMARY KEY(did, rkey),
  UNIQUE (did, post_did, post_rkey)
);
