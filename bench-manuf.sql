
.load target/release/libsqlite3_inet

.output bench-manuf-output.txt

SELECT mac.mac, MAC_FORMAT(mac.mac, 'link-local'), MAC_PREFIX(mac.mac) FROM mac;
