import asyncio, aiohttp.server, socket, os, json

clients = {}

def remove_closed():
    for e in clients:
        clients[e] = list(filter(lambda c: c.transport.get_extra_info("socket").fileno() != -1, clients[e]))

def send_keepalive():
    remove_closed()
    for endpoint in clients:
        for client in clients[endpoint]:
            client.write(b": keepalive\n\n")
    asyncio.get_event_loop().call_later(60, send_keepalive)

class FakeTransport(asyncio.BaseTransport):
    def close(self):
        pass

class HttpServer(aiohttp.server.ServerHttpProtocol):
    @asyncio.coroutine
    def handle_request(self, message, payload):
        response = aiohttp.Response(self.writer, 200, http_version=message.version)
        response.add_header("Content-Type", "text/event-stream; charset=utf-8")
        response.send_headers()
        clients.setdefault(message.path, [])
        clients[message.path] += [response]
        self.transport = FakeTransport()

class UnixServer(asyncio.Protocol):
    def __init__(self, *args, **kwargs):
        super(*args, **kwargs)
        self.data = b""
    def data_received(self, data):
        self.data += data
        lines = self.data.split(b"\n")
        if len(lines) > 1:
            self.data = lines[-1]
            for command in lines[:-1]:
                command = json.loads(command.decode("utf-8"))
                event = ""
                if "event" in command:
                    event += "event: "+str(command["event"])+"\n"
                if "data" in command:
                    for line in command["data"].split("\n"):
                        event += "data: "+str(line)+"\n"
                if "id" in command:
                    event += "id: "+str(command["id"])+"\n"
                event = (event+"\n").encode("utf-8")
                remove_closed()
                for client in clients[command["endpoint"]]:
                    client.write(event)
try:
    os.unlink("/tmp/eventserver.sock")
except FileNotFoundError:
    pass

loop = asyncio.get_event_loop()
httpd = loop.create_server(lambda: HttpServer(), "localhost", 8080)
unixd = loop.create_unix_server(lambda: UnixServer(), path="/tmp/eventserver.sock")
loop.run_until_complete(httpd)
loop.run_until_complete(unixd)
os.chmod("/tmp/eventserver.sock", 0o777)
loop.call_later(60, send_keepalive)
loop.run_forever()

