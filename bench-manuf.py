
import timeit
import sqlite3

db = sqlite3.connect('switch_db_all.sqlite3')
db.enable_load_extension(True)
db.load_extension('target/release/libsqlite3_inet')

cases = [
    'mac.mac',
    'LOWER(mac.mac)',
    'MAC_MANUF(mac.mac)',
    'MAC_MANUF_NATIVE(mac.mac)'
]

for c in cases:
    print(c, timeit.timeit("for _ in db.execute('SELECT mac.mac, " + c + " FROM mac;'): pass", globals=globals(), number=4))
