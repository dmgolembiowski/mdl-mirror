#!/usr/bin/env python

import lmdb
import sys

path = sys.argv[1]
db = sys.argv[2]

env = lmdb.open(path, max_dbs=200)

db = env.open_db(db.encode())
with env.begin() as txn:
    cursor = txn.cursor(db)
    cursor.first()
    for k, v in cursor:
        print((k, v))
