CREATE TABLE "OAuthConnection" (
  "id" TEXT PRIMARY KEY,
  "providerFingerprint" TEXT NOT NULL,
  "clientId" TEXT NOT NULL,
  "ownerSub" TEXT,
  "tokenEncrypted" TEXT NOT NULL,
  "state" TEXT NOT NULL,
  "generation" BIGINT NOT NULL DEFAULT 0,
  "updatedAt" BIGINT NOT NULL
);
