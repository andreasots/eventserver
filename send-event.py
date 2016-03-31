import asyncio
import collections
import json
import logging
import os

log = logging.getLogger("send-event")


class RpcException(Exception):
    pass


class RpcProtocol(asyncio.Protocol):
    def __init__(self):
        self.pending = collections.deque()
        self.transport = None
        self.buffer = b""

    def request(self, function, args, user):
        if self.transport is None:
            raise ConnectionError("Not connected.")
        self.transport.write(json.dumps({
            "command": function,
            "param": args,
            "user": user
        }).encode("utf-8"))
        self.transport.write(b"\n")
        future = asyncio.Future()
        self.pending.append(future)
        return future

    def connection_made(self, transport):
        self.transport = transport

    def connection_lost(self, exc):
        self.eof_received()
        self.transport = None

    def data_received(self, data):
        self.buffer += data
        *messages, self.buffer = self.buffer.split(b'\n')
        for message in messages:
            message = json.loads(message.decode('utf-8'))
            future = self.pending.popleft()
            if message['success']:
                future.set_result(message['result'])
            else:
                future.set_exception(RpcException(message['result']))

    def eof_received(self):
        for future in self.pending:
            future.cancel()
        self.pending.clear()


class RpcClient:
    def __init__(self, protocol):
        self._protocol = protocol

    @classmethod
    async def connect(cls, path, user=None, *args, loop=None, **kwargs):
        if not os.path.isabs(path):
            path = os.path.join(os.environ["XDG_RUNTIME_DIR"], path)
        if loop is None:
            loop = asyncio.get_event_loop()
        transport, protocol = await loop.create_unix_connection(RpcProtocol,
                                                                path, *args,
                                                                **kwargs)
        return cls(protocol)

    def __getattr__(self, name):
        def rpc_method(*args, user=None):
            return self._protocol.request(name, args, user)
        return rpc_method

loop = asyncio.get_event_loop()
client = loop.run_until_complete(RpcClient.connect("eventserver-rpc"))
loop.run_until_complete(client.send_event("/event", "subscriber", "lrrbot"))
loop.run_until_complete(client.register_key("/event", "dickbutt"))
