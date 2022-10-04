import sqlite3
import json
import argparse
from typing import Tuple
from uuid import UUID

def load(path: str):
    with open(path) as ifile:
        data = json.load(ifile)
        return dict((d["uuid"], d["names"]) for d in data)

def merge(data1: dict, data2: dict):
    for _uuid, _names in data2.items():
        _names_0 = data1.get(_uuid)
        if _names_0 is None or len(_names_0) < len(_names):
            data1[_uuid] = _names
    return data1


SQL_QUERY = '''
SELECT "name", "changedToAt", "source"
FROM `names`
WHERE "uuid" = ?
ORDER BY "changedToAt"
'''

SQL_INSERT = '''
INSERT INTO `names`
("uuid", "name", "changedToAt", "source")
VALUES (?, ?, ?, ?)
'''

class Record:

    def __init__(self, data, source: int=None):
        if isinstance(data, dict):
            self.name = data["name"]
            self.changedToAt = data.get("changedToAt")
            self.source = source
        else:
            self.name = str(data[0])
            self.changedToAt = data[1]
            self.source = data[2]

    def params(self, _uuid: UUID):
        return (_uuid.bytes, self.name, self.changedToAt, self.source)

def get_name_history(cursor: sqlite3.Cursor, _uuid: UUID):
    s = cursor.execute(SQL_QUERY, (_uuid.bytes,))
    _names = [Record(r) for r in s]
    _names.sort(key=lambda r: r.changedToAt if r.changedToAt is not None else 0)
    return _names


def main():
    app = argparse.ArgumentParser('merge')
    app.add_argument('-f', '--db-file', required=True)
    app.add_argument('-s', '--source', type=int, default=1, required=False)
    app.add_argument('data', nargs='+')

    args = app.parse_args()

    _source = args.source
    conn = sqlite3.connect(args.db_file)
    
    for p in args.data:
        data = load(p)
        n = len(data)
        c = 0
        print('got data @{}: {}'.format(p, n))
        for _uuid, _names in data.items():
            cursor = conn.cursor()
            c += 1
            _uuid = UUID(_uuid)
            name_history = get_name_history(cursor, _uuid)
            if len(name_history) == 0:
                params = [Record(d, _source).params(_uuid) for d in _names]
                cursor.executemany(SQL_INSERT, params)
            conn.commit()
            print('finished {:.2f}%: {}'.format(c * 100 / n, _uuid))
            pass
    conn.close()

if __name__ == '__main__':
    main()


