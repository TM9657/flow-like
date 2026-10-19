-- Apply before deploying the API that stores archive payloads in the content bucket.
-- Existing inline bundles remain available until they expire.
ALTER TABLE "DeviceArchive" ADD COLUMN "payloadRef" TEXT;
ALTER TABLE "DeviceArchive" ADD COLUMN "manifestJws" TEXT;
ALTER TABLE "DeviceArchive" ADD COLUMN "rosterJws" TEXT;
ALTER TABLE "DeviceArchive" ALTER COLUMN bundle DROP NOT NULL;

-- The migration runner waits for the index build before the next statement.
CREATE INDEX ASYNC "DeviceArchive_ownerId_expiresAt_sizeBytes_idx"
  ON "DeviceArchive"("ownerId", "expiresAt") INCLUDE ("sizeBytes");
ANALYZE "DeviceArchive";
