# Copyright 2026 Sean M. Brennan and contributors
# SPDX-License-Identifier: Apache-2.0
"""AT peer UUID <-> 32-bit LoRa address, learned from announcements, with
conflicts failing closed and persistence across restarts. Twin of C
lora_addrmap."""
import json
import os
import re
import uuid as _uuid

# Only the 36-char hyphenated form, as C uuid_parse accepts; uuid.UUID alone
# would also take braces, urn:uuid: and bare 32-hex.
_UUID_RE = re.compile(r'[0-9a-fA-F]{8}(-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}\Z')
_I63 = 2 ** 63   # jansson integers are signed 64-bit


def _norm(u):
    if not isinstance(u, str) or _UUID_RE.match(u) is None:
        return None
    return str(_uuid.UUID(u)).lower()


class AddrMap:
    MAX = 256

    def __init__(self):
        self._e = {}        # uuid -> [addr, issued_ms]
        self.skipped = []   # the file's keys load() could not take

    def announce(self, uuid, addr, issued_ms, is_member=None) -> str:
        u = _norm(uuid)
        if u is None or type(addr) is not int or type(issued_ms) is not int \
                or not 0 < addr < 0xFFFFFFFF or not 0 <= issued_ms < _I63:
            return 'bad'
        cur = self._e.get(u)
        if cur is not None and issued_ms <= cur[1]:
            return 'stale'
        if cur is None and len(self._e) >= self.MAX:
            return 'full'
        self._e[u] = [addr, issued_ms]
        if is_member is not None:
            # A radio moved from a node no longer on the roster: its old entry
            # goes, rather than conflicting forever with a silent holder.
            for o in [o for o, (a, _) in self._e.items()
                      if o != u and a == addr and not is_member(o)]:
                del self._e[o]
        return 'accepted'

    def _holders(self, addr):
        return sum(1 for a, _ in self._e.values() if a == addr)

    def addr_of(self, uuid):
        e = self._e.get(_norm(uuid))
        return None if e is None or self._holders(e[0]) != 1 else e[0]

    def uuid_of(self, addr):
        if self._holders(addr) != 1:
            return None
        return next(u for u, (a, _) in self._e.items() if a == addr)

    def addr_conflicted(self, addr) -> bool:
        return self._holders(addr) > 1

    def remove(self, uuid) -> bool:
        return self._e.pop(_norm(uuid), None) is not None

    def snapshot(self) -> list:
        return [{'uuid': u, 'addr': a, 'issued_ms': t, 'conflicted': self._holders(a) > 1}
                for u, (a, t) in self._e.items()]

    def save(self, path, self_issued_ms=0) -> None:
        doc = {'version': 1, 'self_issued_ms': int(self_issued_ms),
               'peers': {u: {'addr': a, 'issued_ms': t} for u, (a, t) in self._e.items()}}
        tmp = path + '.tmp'
        try:
            with open(tmp, 'w') as f:
                json.dump(doc, f, separators=(',', ':'))
                f.flush()
                os.fsync(f.fileno())
            os.replace(tmp, path)
        except BaseException:
            try:
                os.remove(tmp)
            except OSError:
                pass
            raise
        # The rename itself is durable once the directory is synced; not every
        # platform can open a directory for that.
        try:
            fd = os.open(os.path.dirname(os.path.abspath(path)), os.O_RDONLY)
        except OSError:
            return
        try:
            os.fsync(fd)
        except OSError:
            pass
        finally:
            os.close(fd)

    @classmethod
    def load(cls, path):
        """(map, self_issued_ms, error). An absent file is not an error; an
        unreadable or malformed one leaves the map empty and says why. A bad
        entry in an otherwise good file is skipped and its key listed in the
        map's ``skipped``."""
        m = cls()
        if not os.path.exists(path):
            return m, 0, None
        try:
            with open(path) as f:
                doc = json.load(f)
            if not isinstance(doc, dict) or type(doc.get('version')) is not int or doc.get('version') != 1 \
                    or not isinstance(doc.get('peers'), dict) \
                    or type(doc.get('self_issued_ms')) is not int \
                    or not 0 <= doc['self_issued_ms'] < _I63:
                raise ValueError('not a lora address map')
            for u, v in doc['peers'].items():
                if not isinstance(v, dict) \
                        or m.announce(u, v.get('addr'), v.get('issued_ms')) != 'accepted':
                    m.skipped.append(u)
            return m, doc['self_issued_ms'], None
        except (OSError, ValueError, AttributeError, TypeError) as err:
            return cls(), 0, str(err)
