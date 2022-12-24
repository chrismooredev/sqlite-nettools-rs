
.load target/release/sqlite3_inet

-- CREATE TABLE test_macs(mac TEXT);
-- -- INSERT INTO test_ouis(mac) VALUES('CC-15-31-19-C8-64');
-- -- INSERT INTO test_ouis(mac) VALUES('00-15-5D-50-CB-F3');
-- -- INSERT INTO test_ouis(mac) VALUES('CE-15-31-19-C8-64');
-- -- INSERT INTO test_ouis(mac) VALUES('CC-15-31-19-C8-68');
-- -- INSERT INTO test_ouis(mac) VALUES('01:00:5e:00:00:16');
-- -- INSERT INTO test_ouis(mac) VALUES('48:a2:e6:22:36:ce');
-- -- INSERT INTO test_ouis(mac) VALUES('cc:15:31:19:c8:64');
-- -- INSERT INTO test_ouis(mac) VALUES('54:83:3a:a1:38:ae');

-- INSERT INTO test_mac(mac, format) VALUES('54:83:3a:a1:38:ae', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('cc:15:31:19:c8:64', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('ff:ff:ff:ff:ff:ff', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('40:5b:d8:6e:23:4d', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('01:00:5e:00:00:16', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('01:00:5e:7f:ff:fa', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('04:c9:d9:bf:03:2f', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('f8:0d:ac:af:74:59', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('01:00:5e:00:00:fb', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('33:33:00:00:00:fb', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('fc:69:47:7c:e5:07', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('48:a2:e6:22:36:ce', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('b8:2c:a0:0c:d4:64', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('44:67:55:08:65:5a', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('20:d7:78:cd:6f:ae', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('d4:12:43:c5:56:f6', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('b8:d7:af:8f:b4:bd', 'dash');
-- INSERT INTO test_mac(mac, format) VALUES('44:91:60:c4:e6:f1', 'dash');

CREATE TABLE mac_test (
	mac	TEXT,
	format	TEXT
);
INSERT INTO mac_test(mac, format) VALUES ('54:83:3a:a1:38:ae', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('cc:15:31:19:c8:64', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('ff:ff:ff:ff:ff:ff', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('40:5b:d8:6e:23:4d', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('01:00:5e:00:00:16', '');
INSERT INTO mac_test(mac, format) VALUES ('01:00:5e:7f:ff:fa', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('04:c9:d9:bf:03:2f', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('f8:0d:ac:af:74:59', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('01:00:5e:00:00:fb', 'hex');
INSERT INTO mac_test(mac, format) VALUES ('33:33:00:00:00:fb', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('fc:69:47:7c:e5:07', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('48:a2:e6:22:36:ce', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('b8:2c:a0:0c:d4:64', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('44:67:55:08:65:5a', 'link-local');
INSERT INTO mac_test(mac, format) VALUES ('20:d7:78:cd:6f:ae', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('d4:12:43:c5:56:f6', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('b8:d7:af:8f:b4:bd', 'dash');
INSERT INTO mac_test(mac, format) VALUES ('44:91:60:c4:e6:f1', 'dash');
-- SELECT mac, format, MAC_FORMAT(mac, format) FROM mac_test;
SELECT mac, format, mac MAC_FORMAT format FROM mac_test;

-- SELECT mac, MAC_PREFIX(mac), MAC_MANUF(mac), MAC_MANUFLONG(mac), MAC_COMMENT(mac) FROM test_ouis;

-- SELECT mac, MAC_FORMAT(mac), MAC_FORMAT(mac, 'hex'), MAC_FORMAT(mac, 'dot'), MAC_FORMAT(mac, 'canonical'), MAC_FORMAT(mac, NULL), MAC_FORMAT(mac, 'interface-id'), MAC_FORMAT(mac, 'link-local') FROM test_ouis;

-- SELECT mac, MAC_ISUNICAST(mac), MAC_ISMULTICAST(mac), MAC_ISUNIVERSAL(mac), MAC_ISLOCAL(mac) FROM test_ouis;

-- SELECT mac, MAC_MANUF(mac) FROM test_macs;
