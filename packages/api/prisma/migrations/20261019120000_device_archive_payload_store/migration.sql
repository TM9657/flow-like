ALTER TABLE "DeviceArchive" ADD COLUMN "payloadRef" TEXT;
ALTER TABLE "DeviceArchive" ADD COLUMN "manifestJws" TEXT;
ALTER TABLE "DeviceArchive" ADD COLUMN "rosterJws" TEXT;
ALTER TABLE "DeviceArchive" ALTER COLUMN bundle DROP NOT NULL;

CREATE INDEX "DeviceArchive_ownerId_expiresAt_sizeBytes_idx"
  ON "DeviceArchive"("ownerId", "expiresAt", "sizeBytes");
