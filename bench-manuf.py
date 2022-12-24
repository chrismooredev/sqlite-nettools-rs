
import timeit
import sqlite3
import sys

db = sqlite3.connect('switch_db_all.sqlite3')
db.enable_load_extension(True)

db = sqlite3.connect(':memory:')
db.enable_load_extension(True)
if sys.platform in ['win32', 'cygwin']:
    db.load_extension('./target/release/sqlite3_inet')
else:
    db.load_extension('./target/release/libsqlite3_inet')

cases = [
    # "MAC_FORMAT(mac.mac)",
    # "MAC_FORMAT(mac.mac, NULL)",
    # "MAC_FORMAT(mac.mac, \\'colon\\')",
    # "MAC_FORMAT(mac.mac, \\'link-local\\')",
    # "MAC_PREFIX(mac.mac)",
    # "MAC_PREFIX(mac.mac)",
    # 'mac.mac',
    # 'LOWER(mac.mac)',
    # 'MAC_MANUF(mac.mac)',
    # 'MAC_MANUF_NATIVE(mac.mac)'
    'MAC_FORMAT(mac.mac), MAC_PREFIX(mac.mac)'
]

for c in cases:
    print(c, timeit.timeit("for _ in db.execute('SELECT mac.mac, " + c + " FROM mac;'): pass", globals=globals(), number=20))
