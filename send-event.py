import asyncio
import logging
import os

import msgpack

log = logging.getLogger("send-event")


class RpcException(Exception):
    pass


class RpcProtocol(asyncio.Protocol):
    def __init__(self):
        self.counter = 0
        self.pending = {}
        self.transport = None

        self.partial = [None, []]

    def request(self, function, args):
        if self.transport is None:
            raise ConnectionError("Not connected.")
        self.transport.write(msgpack.packb([0, self.counter, function, args],
                                           use_bin_type=True))
        future = self.pending[self.counter] = asyncio.Future()

        while True:
            self.counter = (self.counter + 1) & 0xFFFFFFFF
            if self.counter not in self.pending:
                break

        return future

    def connection_made(self, transport):
        self.transport = transport

    def connection_lost(self, exc):
        self.eof_received(exc)
        self.transport = None

    def data_received(self, data):
        self.unpacker.feed(data)
        while True:
            if self.partial[0] is None:
                try:
                    self.partial[0] = self.unpacker.read_array_header()
                except msgpack.OutOfData:
                    return
            elif len(self.partial[1]) < self.partial[0]:
                try:
                    self.partial[1].append(self.unpacker.unpack())
                except msgpack.OutOfData:
                    return
            else:
                msgtype, *_ = self.partial[1]
                if msgtype == 1:
                    msgtype, msgid, err, result = self.partial[1]
                    if err is not None:
                        self.pending[msgid].set_exception(RpcException(err))
                    else:
                        self.pending[msgid].set_result(result)
                    del self.pending[msgid]
                else:
                    log.warn("Unrecognised RPC response type: %d, message: %r",
                             msgtype, self.partial[1][1:])
                self.partial = [None, []]

    def eof_received(self):
        for future in self.pending.values():
            future.cancel()
        self.pending.clear()


class RpcClient:
    def __init__(self, protocol):
        self._protocol = protocol

    @classmethod
    async def connect(cls, path, *args, loop=None, **kwargs):
        if not os.path.isabs(path):
            path = os.path.join(os.environ["XDG_RUNTIME_DIR"], path)
        if loop is None:
            loop = asyncio.get_event_loop()
        transport, protocol = await loop.create_unix_connection(RpcProtocol,
                                                                path, *args,
                                                                **kwargs)
        return cls(protocol)

    def __getattr__(self, name):
        return lambda *args: self._protocol.request(name, args)

loop = asyncio.get_event_loop()
client = loop.run_until_complete(RpcClient.connect("eventserver-rpc"))
loop.run_until_complete(client.send_event("/event", "subscriber", "lrrbot"))
