-- Accounts
CREATE TABLE accounts(
  did TEXT PRIMARY KEY,
  handle TEXT,
  rev TEXT,
  is_active BOOLEAN NOT NULL DEFAULT true,
  status TEXT NOT NULL DEFAULT 'active',
  display_name TEXT,
  pronouns TEXT,
  avatar_blob_cid TEXT,
  created_at BIGINT NOT NULL,
  indexed_at BIGINT NOT NULL DEFAULT (extract(epoch from now()) * 1000)::BIGINT
);

-- Labels
CREATE TYPE labeler_behaviour AS ENUM ('annotate', 'moderate');
CREATE TYPE labeler_behaviour_setting AS ENUM ('ignore', 'inform', 'warn', 'hide');
CREATE TABLE labeler_rules(
  did TEXT NOT NULL REFERENCES accounts(did) ON DELETE CASCADE,
  rkey TEXT NOT NULL, -- This is also the rule name
  name TEXT NOT NULL,
  description TEXT NOT NULL,
  behaviour labeler_behaviour NOT NULL,
  -- Annotate
  default_setting labeler_behaviour_setting,
  adult_content BOOLEAN,
  -- Moderate
  takedown BOOLEAN,
  created_at BIGINT NOT NULL,
  edited_at BIGINT,
  indexed_at BIGINT NOT NULL DEFAULT (extract(epoch from now()) * 1000)::BIGINT,
  PRIMARY KEY (did, rkey),
  CHECK (
    CASE behaviour
      WHEN 'annotate' THEN 
        default_setting IS NOT NULL AND 
        adult_content IS NOT NULL AND 
        takedown IS NULL
      WHEN 'moderate' THEN 
        default_setting IS NULL AND 
        adult_content IS NULL AND 
        takedown IS NOT NULL
    END
  )
);
CREATE TABLE labels(
  rkey TEXT NOT NULL,
  did TEXT NOT NULL REFERENCES accounts(did) ON DELETE CASCADE,
  rule_did TEXT NOT NULL,
  rule_rkey TEXT NOT NULL,
  subject_did TEXT NOT NULL,
  subject_collection TEXT,
  subject_rkey TEXT,
  reason TEXT,
  created_at BIGINT NOT NULL,
  expires_at BIGINT,
  edited_at BIGINT,
  indexed_at BIGINT NOT NULL DEFAULT (extract(epoch from now()) * 1000)::BIGINT,
  PRIMARY KEY (did, rkey),
  CHECK (
    (subject_collection IS NULL AND subject_rkey IS NULL) OR
    (subject_collection IS NOT NULL AND subject_rkey IS NOT NULL)
  )
);

-- Posts
CREATE TABLE posts(
  did TEXT NOT NULL REFERENCES accounts(did) ON DELETE CASCADE,
  rkey TEXT NOT NULL,
  title TEXT NOT NULL,
  tags TEXT[],
  languages TEXT[],
  media_blob_cid TEXT NOT NULL,
  media_blob_mime TEXT NOT NULL,
  media_blob_alt TEXT,
  media_blob_width INTEGER NOT NULL,
  media_blob_height INTEGER NOT NULL,
  created_at BIGINT NOT NULL,
  edited_at BIGINT,
  indexed_at BIGINT NOT NULL DEFAULT (extract(epoch from now()) * 1000)::BIGINT,
  PRIMARY KEY(did, rkey)
);
CREATE TABLE post_favourites(
  did TEXT NOT NULL REFERENCES accounts(did) ON DELETE CASCADE,
  rkey TEXT NOT NULL,
  post_did TEXT NOT NULL,
  post_rkey TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  indexed_at BIGINT NOT NULL DEFAULT (extract(epoch from now()) * 1000)::BIGINT,
  PRIMARY KEY(did, rkey),
  UNIQUE (did, post_did, post_rkey)
);
