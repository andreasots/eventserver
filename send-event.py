import socket, json

sse = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sse.connect("/tmp/eventserver.sock")

sse.send(json.dumps({
    "endpoint": "/event",
    "event": "subscriber",
    "data": "lrrbot",
    "id": "12345"
}).encode("utf-8")+b"\n")

sse.send(json.dumps({
    "endpoint": "/event",
    "event": "event",
    "data": "data1\ndata2",
    "id": "12346"
}).encode("utf-8")+b"\n")

sse.close()
