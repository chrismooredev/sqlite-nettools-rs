
import sqlite3
import os
import sys
import subprocess

os.environ['SQLITE3_LIB_DIR'] = 'sqlite3'
os.environ['SQLITE3_INCLUDE_DIR'] = 'sqlite3'
subprocess.run(['cargo', 'build', '--release'], check=True)

db = sqlite3.connect(':memory:')
db.enable_load_extension(True)
if sys.platform in ['win32', 'cygwin']:
    db.load_extension('./target/release/sqlite3_inet')
else:
    db.load_extension('./target/release/libsqlite3_inet')

cur = db.executescript("""
CREATE TABLE "mac_test" (
	"mac"	TEXT,
	"format"	TEXT
);
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('54:83:3a:a1:38:ae', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('cc:15:31:19:c8:64', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('ff:ff:ff:ff:ff:ff', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('40:5b:d8:6e:23:4d', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('01:00:5e:00:00:16', '');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('01:00:5e:7f:ff:fa', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('04:c9:d9:bf:03:2f', NULL);
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('f8:0d:ac:af:74:59', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('01:00:5e:00:00:fb', 'hex');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('33:33:00:00:00:fb', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('fc:69:47:7c:e5:07', 'dah');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('48:a2:e6:22:36:ce', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('b8:2c:a0:0c:d4:64', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('44:67:55:08:65:5a', 'link-local');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('20:d7:78:cd:6f:ae', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('d4:12:43:c5:56:f6', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('b8s:d7:af:8f:zb4:bd', 'dash');
INSERT INTO "main"."mac_test" ("mac", "format") VALUES ('44:91:60:c4:e6:f1', 'dash');

-- SELECT mac, format, mac MAC_FORMAT format FROM mac_test;
""")
for i, ent in enumerate(db.execute("SELECT mac, format, MAC_FORMAT(mac, '?~' || format) FROM mac_test;")):
    print(i, ent)

TEST_CASES = [
    "MAC_MANUF('3c-a6-f6-c4-34-f8') IS 'Apple'",
    "MAC_MANUF('8c-1c-da-82-4c-2e') IS 'Atol'",
]

for i, case in enumerate(TEST_CASES):
    res = list(db.execute('SELECT ? WHERE ' + case, [case]))
    if len(res) != 1:
        print(f'[error][test case {i}] {repr(case)}')

# SELECT 'manuf apple' WHERE MAC_MANUF('3c-a6-f6-c4-34-f8') IS NOT 'Apple';
# SELECT 'manuf atol' WHERE MAC_MANUF('8c-1c-da-82-4c-2e') IS NOT 'Atol';
# -- 8C:1C:DA:80
