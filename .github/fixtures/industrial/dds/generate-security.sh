#!/bin/sh
set -eu
# Test identities are generated for each fixture build and never used outside E2E tests.
umask 077
out="$1"
mkdir -p "$out"
cd "$out"
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem -sha256 -days 30 -subj /CN=FlowLikeDDSTestCA >/dev/null 2>&1
openssl req -x509 -newkey rsa:2048 -nodes -keyout wrong-ca.key -out wrong-ca.pem -sha256 -days 30 -subj /CN=UntrustedDDSTestCA >/dev/null 2>&1
for name in flow-like peer; do
    openssl req -newkey rsa:2048 -nodes -keyout "$name.key" -out "$name.csr" -subj "/CN=$name" >/dev/null 2>&1
    openssl x509 -req -in "$name.csr" -CA ca.pem -CAkey ca.key -CAcreateserial -out "$name.pem" -days 30 -sha256 >/dev/null 2>&1
done
openssl x509 -req -in flow-like.csr -CA wrong-ca.pem -CAkey wrong-ca.key -CAcreateserial -out untrusted.pem -days 30 -sha256 >/dev/null 2>&1
cat > governance.xml <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<dds><domain_access_rules><domain_rule>
<domains><id_range><min>0</min><max>230</max></id_range></domains>
<allow_unauthenticated_participants>false</allow_unauthenticated_participants>
<enable_join_access_control>true</enable_join_access_control>
<discovery_protection_kind>ENCRYPT</discovery_protection_kind>
<liveliness_protection_kind>ENCRYPT</liveliness_protection_kind>
<rtps_protection_kind>ENCRYPT</rtps_protection_kind>
<topic_access_rules><topic_rule><topic_expression>*</topic_expression>
<enable_discovery_protection>true</enable_discovery_protection>
<enable_liveliness_protection>true</enable_liveliness_protection>
<enable_read_access_control>true</enable_read_access_control>
<enable_write_access_control>true</enable_write_access_control>
<metadata_protection_kind>ENCRYPT</metadata_protection_kind>
<data_protection_kind>ENCRYPT</data_protection_kind>
</topic_rule></topic_access_rules>
</domain_rule></domain_access_rules></dds>
XML
sed 's/>ENCRYPT</>NONE</g' governance.xml > governance-none.xml
sed 's/>ENCRYPT</>SIGN</g' governance.xml > governance-sign.xml
for name in flow-like peer denied; do
    subject="$name"
    topic='secure_*'
    if [ "$name" = denied ]; then subject=flow-like; topic=forbidden; fi
    cat > "$name-permissions.xml" <<XML
<?xml version="1.0" encoding="UTF-8"?>
<dds><permissions><grant name="$name">
<subject_name>CN=$subject</subject_name>
<validity><not_before>2020-01-01T00:00:00</not_before><not_after>2099-01-01T00:00:00</not_after></validity>
<allow_rule><domains><id_range><min>0</min><max>230</max></id_range></domains>
<publish><topics><topic>$topic</topic></topics></publish>
<subscribe><topics><topic>$topic</topic></topics></subscribe>
</allow_rule><default>DENY</default>
</grant></permissions></dds>
XML
done
for name in governance governance-none governance-sign flow-like-permissions peer-permissions denied-permissions; do
    openssl smime -sign -text -md sha256 -in "$name.xml" -out "$name.p7s" -signer ca.pem -inkey ca.key >/dev/null 2>&1
done
cp ca.pem permissions-ca.pem
mkdir -p untrusted
cp wrong-ca.pem untrusted/ca.pem
cp ca.pem untrusted/permissions-ca.pem
cp peer.key governance.p7s peer-permissions.p7s untrusted/
openssl x509 -req -in peer.csr -CA wrong-ca.pem -CAkey wrong-ca.key -CAcreateserial -out untrusted/peer.pem -days 30 -sha256 >/dev/null 2>&1
chmod 755 untrusted
chmod 644 untrusted/*
# The runtime image runs the fixture as an unprivileged user. These are disposable test secrets.
chmod 755 "$out"
chmod 644 ./*.pem ./*.key ./*.p7s
